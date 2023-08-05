use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal::SIGTRAP;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::convert::TryInto;
use std::mem::size_of;
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
#[derive(Clone)]
pub struct Breakpoint {
    addr: usize,
    orig_byte: u8,
}

pub struct Inferior {
    child: Child,
    breakpoints: HashMap<usize, Breakpoint>,
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
                WaitStatus::Stopped(_pid, signal) => println!("signal {}", signal),
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
        Some(Inferior {
            child,
            breakpoints: HashMap::new(),
        })
    }
    pub fn goon(&self) -> Result<Status, nix::Error> {
        //get register id. see whether it has been stop.

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
        // let line_number = debug_data.get_line_from_addr(reg.rip as usize).unwrap();
        // let function_name = debug_data.get_function_from_addr(reg.rip as usize).unwrap();
        // println!("{} ({})", function_name, line_number);
        let mut instruction_ptr = reg.rip as usize;
        let mut base_ptr = reg.rbp;
        while true {
            let line_number = debug_data.get_line_from_addr(instruction_ptr);
            let function_name = debug_data.get_function_from_addr(instruction_ptr);
            match line_number {
                Some(line) => match function_name {
                    Some(function) => {
                        println!("{} ({})", function, line);
                        if function == "main" {
                            break;
                        }
                    }
                    None => {
                        return Ok(());
                    }
                },
                None => {
                    return Ok(());
                }
            }

            instruction_ptr =
                ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType)? as u64;
        }
        Ok(())
    }

    pub fn breakpoint(&mut self, addr: &usize) -> Result<(), nix::Error> {
        if self.breakpoints.contains_key(addr) {
            return Ok(());
        }
        let orig_val = self.write_byte(addr.clone(), 0xcc)?;
        let b = Breakpoint {
            addr: addr.clone(),
            orig_byte: orig_val,
        };
        //TODO: Might need to handle if k exist.
        self.breakpoints.insert(addr.clone(), b);

        Ok(())
    }
    pub fn continue_from_breakpoint(&mut self, addr: &usize) -> Result<(), nix::Error> {
        //write back to addr
        self.recover(addr)?;
        //rip rewind
        //println!("get reg");
        let mut regs = ptrace::getregs(self.pid())?;
        //println!("pre {}", regs.rip);
        regs.rip = regs.rip - 1;
        //println!("set reg");
        ptrace::setregs(self.pid(), regs)?;
        //step
        //println!("step");
        ptrace::step(self.pid(), None)?;
        self.wait(None)?;
        // let regs = ptrace::getregs(self.pid())?;
        // println!("post {}", regs.rip);
        //write back
        //println!("breakpoint");
        self.breakpoint(addr)?;
        Ok(())
    }
    pub fn recover(&mut self, addr: &usize) -> Result<(), nix::Error> {
        //println!("recover address: {}", addr);
        let bp = match self.breakpoints.get(addr) {
            Some(bp) => bp,
            None => {
                return Err(nix::Error::Sys(nix::errno::Errno::EBADE));
            }
        };
        let a = bp.clone();
        self.breakpoints.remove(addr);
        self.write_byte(a.addr.clone(), a.orig_byte.clone())?;
        Ok(())
    }
    pub fn find_break_point(&self, debug_data: &DwarfData) -> Option<usize> {
        let reg = ptrace::getregs(self.pid()).expect("get regegister failed");
        let rip = reg.rip as usize;
        let bp_addr_cand = rip - 1;
        //println!("current_location: {}", bp_addr_cand);
        if self.breakpoints.contains_key(&bp_addr_cand) {
            return Some(bp_addr_cand);
        }
        None
    }
    pub fn _step_one(&self, _addr: &usize) -> Result<(), nix::Error> {
        ptrace::step(self.pid(), SIGTRAP)?;

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

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

impl Inferior {
    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }
}
