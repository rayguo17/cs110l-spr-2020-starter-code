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
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        let infer = self.inferior.as_mut().unwrap();
                        infer.goon().unwrap();
                        self.wait_thread();
                        // match infer.wait(None) {
                        //     Ok(status) => match status {
                        //         crate::inferior::Status::Exited(exit_code) => {
                        //             println!("Child exited (status {})", exit_code);
                        //         }

                        //         other => {
                        //             println!("Unexpected status of child process.");
                        //         }
                        //     },
                        //     Err(e) => println!("{}", e),
                        // };
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => match &self.inferior {
                    Some(infer) => match infer.goon() {
                        Ok(status) => {
                            self.wait_thread();
                        }
                        Err(e) => {}
                    },
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
            }
        }
    }

    fn wait_thread(&mut self) {
        let infer = self.inferior.as_ref().unwrap();
        match infer.wait(None).expect("encounter error when waiting") {
            crate::inferior::Status::Stopped(signal, _) => {
                println!("Child stopped (signal {})", signal);
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
