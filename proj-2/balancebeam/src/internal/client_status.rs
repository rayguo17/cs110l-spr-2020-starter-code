use std::{
    collections::HashMap,
    sync::Arc,
    sync::Mutex,
    thread::{self, JoinHandle},
    time,
};

use crossbeam_channel::{Receiver, Sender};

pub struct Client {
    ip: String,
    times: u64,
    last_modify_time: time::SystemTime,
}
pub struct Command {
    pub res_send: Sender<bool>,
    pub cmd: String,
}
//should it be per ip? or just general?

pub struct ClientManager {
    store: Arc<Mutex<HashMap<String, Client>>>,
    cmd_recv: Receiver<Command>,
    pub cmd_send: Sender<Command>,
}
//should have a main routine handling the inner connection.
// maybe we should able to see how database or other stuff handle this kind of situation?

impl ClientManager {
    pub fn new() -> ClientManager {
        let map = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = crossbeam_channel::unbounded();
        return ClientManager {
            store: map,
            cmd_recv: receiver,
            cmd_send: sender,
        };
    }
    pub fn inner_routine_invoker(&self, max_request: usize) -> JoinHandle<()> {
        let state = self.store.clone();
        let recv = self.cmd_recv.clone();
        let handle = thread::spawn(move || Self::inner_routine(recv, state, max_request));
        return handle;
    }
    pub fn inner_routine(
        recv: Receiver<Command>,
        state: Arc<Mutex<HashMap<String, Client>>>,
        max_request: usize,
    ) {
        loop {
            match recv.recv_timeout(std::time::Duration::from_millis(20000)) {
                Ok(m) => {
                    let mut state_inner = state.lock().unwrap();
                    let ip = m.cmd.clone();
                    if !state_inner.contains_key(&ip) {
                        let c = Client {
                            ip: ip.clone(),
                            times: 1,
                            last_modify_time: time::SystemTime::now(),
                        };
                        state_inner.insert(ip.clone(), c);
                        m.res_send.send(true);
                        continue;
                    }
                    let mut c = match state_inner.get_mut(&ip) {
                        Some(c) => c,
                        None => {
                            m.res_send.send(false);
                            continue;
                        } //should handle if not get it.
                    };
                    if Self::available_calc(c, max_request) {
                        m.res_send.send(true);
                        continue;
                    }
                    m.res_send.send(false);
                }
                Err(e) => {
                    match e {
                        crossbeam_channel::RecvTimeoutError::Disconnected => {
                            break;
                        }
                        crossbeam_channel::RecvTimeoutError::Timeout => {
                            // do garbage collection check.
                        }
                    }
                }
            }
            //do annual check... before accepting more time out.
        }
    }
    fn available_calc(c: &mut Client, max_request: usize) -> bool {
        if max_request == 0 {
            return true;
        }
        //first compare current time with last_modify time, if they are in the same interval then compare the max_request, else clean to 0 restart the counter.
        let now = time::SystemTime::now();
        //per minute
        if now.duration_since(c.last_modify_time).unwrap() > time::Duration::from_secs(60) {
            c.last_modify_time = now;
            c.times = 1;
            return true;
        } else {
            if c.times < max_request as u64 {
                c.times = c.times + 1;
                return true;
            }
        }

        return false;
    }
    fn _garbage_collect_(state: Arc<Mutex<HashMap<String, Client>>>) {}
    fn _check_availability(&mut self, ip: &String) -> bool {
        // if !self.store.contains_key(ip) {
        //     //create a new one and then insert it.
        //     //should have a active checker to clean up the client manager memory
        //     //
        //     let c = Client {
        //         ip: ip.clone(),
        //         times: 1,
        //     };
        //     self.store.insert(ip.clone(), c);
        //     return true;
        // }
        // //how to remain order for this things?
        return true;
    }
}
