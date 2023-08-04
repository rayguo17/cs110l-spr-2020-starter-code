use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal::SIGTRAP;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::convert::TryInto;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;

use crate::dwarf_data::DwarfData;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),

    Continued,
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        // TODO: implement me!
        //can not put it like this: let cmd = Command::new(program).args(args)
        //because, Command::new() create a new instance of Command
        //but it is not owned by anyone, it is a temporary variable
        //what args return is a reference to the temporary variable. it will not live long enough.
        let mut cmd = Command::new(target);
        cmd.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        //Error:should we handle error?
        let child = cmd.spawn().expect("child spawn failed.");
        let pid = nix::unistd::Pid::from_raw(child.id() as i32);
        match waitpid(pid, None) {
            Ok(a) => match a {
                WaitStatus::Stopped(_pid, signal) => (),
                other => {
                    println!("child process not stopped!");
                    return None;
                }
            },
            Err(e) => {
                println!("encouter error when waiting");
                return None;
            }
        }
        Some(Inferior { child })
    }
    pub fn goon(&self) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), None)?;
        Ok(Status::Continued)
    }
    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }
    pub fn kill(&mut self) -> Result<Status, nix::Error> {
        println!("Killing running inferior (pid {})", self.pid());
        self.child.kill().unwrap();
        match self.wait(None)? {
            Status::Signaled(_) => Ok(Status::Exited(0)),
            other => Err(nix::Error::InvalidUtf8),
        }

        // match &self.child.kill() {
        //     Ok(()) => Ok(Status::Exited(0)),
        //     Err(e) => ,
        // }
    }
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        //println!("Hello world");
        let reg = ptrace::getregs(self.pid())?;
        println!("reg.rip as usize {}", reg.rip as usize);
        let line_number = debug_data.get_line_from_addr(reg.rip as usize).unwrap();
        let function_name = debug_data.get_function_from_addr(reg.rip as usize).unwrap();
        println!("{} ({})", function_name, line_number);
        Ok(())
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }
}
