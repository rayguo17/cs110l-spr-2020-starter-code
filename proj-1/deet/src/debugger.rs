use std::ops::Index;

use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::Inferior;
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    dwarf_data: DwarfData,
    break_points: Vec<usize>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        debug_data.print();
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        //() is unit type, when we are doing something like println!() , we are implicitly returning () type.
        //here it means there has no helper or use default helper.

        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path); //store inside Editor...

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            dwarf_data: debug_data,
            break_points: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if let Some(infer) = &mut self.inferior {
                        //Error handling of error of kill.
                        println!("program exist, killing and restarting.");
                        infer.kill().expect("kill failed");
                        //Dropped the variable.
                        self.inferior = None;
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        let infer = self.inferior.as_mut().unwrap();
                        for b in self.break_points.iter() {
                            infer.breakpoint(b).unwrap();
                        }

                        infer.goon().unwrap();
                        self.wait_thread();
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => match &mut self.inferior {
                    //can we judge from here?
                    Some(infer) => {
                        match infer.find_break_point(&self.dwarf_data) {
                            Some(addr) => {
                                match infer.continue_from_breakpoint(&addr) {
                                    Ok(a) => {}
                                    Err(e) => {
                                        println!("err when continue from breakpoint {}", e);
                                    }
                                };
                                println!("found current location correspond with breakpoint.")
                            }
                            None => {
                                println!("addr not found!");
                            }
                        };
                        match infer.goon() {
                            Ok(_status) => {
                                self.wait_thread();
                            }
                            Err(_e) => {}
                        }
                    }
                    None => {
                        println!("Run the program first!");
                    }
                },
                DebuggerCommand::BackTrace => match &self.inferior {
                    Some(infer) => {
                        infer.print_backtrace(&self.dwarf_data).expect("msg");
                    }
                    None => {
                        println!("Run the program first!");
                    }
                },
                DebuggerCommand::Quit => {
                    if let Some(infer) = &mut self.inferior {
                        //Error handling of error of kill.
                        println!("program exist, killing");
                        infer.kill().expect("kill failed");
                        //Dropped the variable.
                        self.inferior = None;
                    }
                    return;
                }
                DebuggerCommand::Break(addr_str) => {
                    let addr_usize = self.str_to_addr(addr_str).unwrap();
                    // setup multiple kind of addr should interpret it to usize addr

                    // let mut char_iter = arg.chars();
                    // if char_iter.next().unwrap() == '*' {
                    //     let remain = char_iter.as_str().to_string();
                    //     return Some(DebuggerCommand::Break(remain));
                    // }

                    //first store the addr_usize into the break vec, should check contains first.
                    if !self.break_points.contains(&addr_usize) {
                        self.break_points.push(addr_usize.clone()); //ownership transfer??
                    }
                    //if already run program, we set the break point immediately
                    if let Some(infer) = &mut self.inferior {
                        //Error handling of error of break.
                        println!("program exist, loading break points");
                        infer.breakpoint(&addr_usize).expect("kill failed");
                    }
                    let mut index = 0;
                    let mut cnt = 0;
                    for pt in self.break_points.iter() {
                        if *pt == addr_usize {
                            index = cnt;
                            break;
                        }
                        cnt = cnt + 1;
                    }
                    println!("Set breakpoint {} at {}", index, addr_usize);
                }
            }
        }
    }
    fn parse_address(&self, addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }
    fn str_to_addr(&self, str: String) -> Option<usize> {
        match usize::from_str_radix(&str, 10) {
            Ok(line) => match self.dwarf_data.get_addr_for_line(None, line) {
                Some(addr) => {
                    println!("addr from line: {}", addr);
                    return Some(addr);
                }
                None => {}
            },
            Err(e) => {}
        };
        let mut char_iter = str.chars();
        if char_iter.next().unwrap() == '*' {
            let remain = char_iter.as_str().to_string();
            return Some(self.parse_address(&remain).unwrap());
        }
        match self.dwarf_data.get_addr_for_function(None, &str) {
            Some(addr) => {
                return Some(addr);
            }
            None => {}
        }
        None
    }
    fn wait_thread(&mut self) {
        let infer = self.inferior.as_ref().unwrap();
        match infer.wait(None).expect("encounter error when waiting") {
            crate::inferior::Status::Stopped(signal, instruction_ptr) => {
                println!("Child stopped (signal {})", signal);
                let line_number = self.dwarf_data.get_line_from_addr(instruction_ptr);
                match line_number {
                    Some(line) => println!("Stopped at {}", line),
                    None => {}
                }
            }
            crate::inferior::Status::Exited(exit_code) => {
                println!("Child exited (status {})", exit_code);
                self.inferior = None;
            }
            crate::inferior::Status::Signaled(signal) => {
                println!("Child signaled (signal {})", signal);
                self.inferior = None;
            }
            crate::inferior::Status::Continued => {}
        }
    }
    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
