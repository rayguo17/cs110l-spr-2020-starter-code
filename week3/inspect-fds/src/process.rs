use crate::open_file::OpenFile;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: usize,
    pub ppid: usize,
    pub command: String,
}

impl Process {
    pub fn new(pid: usize, ppid: usize, command: String) -> Process {
        Process { pid, ppid, command }
    }
    pub fn print(&self){
        print!(
            "{0:=^70}\n",
            format!(
                "\"{}\" (pid {}, ppid {})",
                self.command,self.pid,self.ppid
            )
        );
        let res =  self.list_open_files();
        match res{
            Some(fds)=>{
                for (fd,file) in fds.iter(){
                    print!("{:<4} {:<15} cursor: {:<4} {}\n",
                        fd,
                        format!("({})",file.access_mode),
                        file.cursor,
                        file.colorized_name(),
                    )
                }
                println!()
            },
            None=>println!("Warning: could not inspect file descriptors for this process!")
        }
    }

    /// This function returns a list of file descriptor numbers for this Process, if that
    /// information is available (it will return None if the information is unavailable). The
    /// information will commonly be unavailable if the process has exited. (Zombie processes
    /// still have a pid, but their resources have already been freed, including the file
    /// descriptor table.)
    pub fn list_fds(&self) -> Option<Vec<usize>> {
        // TODO: implement for Milestone 3
           let entries = fs::read_dir(format!("/proc/{}/fd", self.pid)).ok()?;
        // {
        //     Ok(read_dir)=>read_dir,
        //     Err(e)=>return None
        // };
        let mut res = Vec::new();
        for x in entries {
            match x {
                Ok(de)=> {
                    //println!("{}", de.file_name().to_str()?) ;
                    match de.file_name().to_str() {
                        Some(fd_str)=> {
                            match fd_str.parse::<usize>(){
                                Ok(fd)=>res.push(fd),
                                Err(_e)=>return None
                            }
                        },
                        None=>{
                            println!("Got none dir");
                            continue
                        }
                    }

                },
                Err(_e)=>return None
            }
        }
        if res.len()==0{
            None
        }else{
            Some(res)
        }
        // unimplemented!();
    }

    /// This function returns a list of (fdnumber, OpenFile) tuples, if file descriptor
    /// information is available (it returns None otherwise). The information is commonly
    /// unavailable if the process has already exited.
    pub fn list_open_files(&self) -> Option<Vec<(usize, OpenFile)>> {
        let mut open_files = vec![];
        for fd in self.list_fds()? {
            open_files.push((fd, OpenFile::from_fd(self.pid, fd)?));
        }
        Some(open_files)
    }
}

#[cfg(test)]
mod test {
    use crate::ps_utils;
    use std::process::{Child, Command};
    use std::{thread, time};
    use nix::libc::sleep;

    fn start_c_program(program: &str) -> Child {
        Command::new(program)
            .spawn()
            .expect(&format!("Could not find {}. Have you run make?", program))
    }

    #[test]
    fn test_list_fds() {
        let mut test_subprocess = start_c_program("./multi_pipe_test");
        thread::sleep(time::Duration::from_millis(500));
        let process = ps_utils::get_target("multi_pipe_test").unwrap().unwrap();
        assert_eq!(
            process
                .list_fds()
                .expect("Expected list_fds to find file descriptors, but it returned None"),
            vec![0, 1, 2, 4, 5]
        );
        let _ = test_subprocess.kill();
    }

    #[test]
    fn test_list_fds_zombie() {
        let mut test_subprocess = start_c_program("./nothing");
        let process = ps_utils::get_target("nothing").unwrap().unwrap();
        assert!(
            process.list_fds().is_none(),
            "Expected list_fds to return None for a zombie process"
        );
        let _ = test_subprocess.kill();
    }
}
