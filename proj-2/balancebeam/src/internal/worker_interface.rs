use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;

use super::{client_status::Command, proxy_status::UpstreamStatus};
pub struct WorkerInterface {
    cmd_sender: Sender<Command>,
    state: Arc<RwLock<UpstreamStatus>>,
}

impl WorkerInterface {
    pub fn new(cmd_sender: Sender<Command>, state: Arc<RwLock<UpstreamStatus>>) -> WorkerInterface {
        return WorkerInterface {
            cmd_sender: cmd_sender,
            state,
        };
    }
    pub fn select_random_updastream(&self) -> Option<String> {
        return self.state.read().unwrap().select_random_updastream();
    }
    pub fn noti_fail(&self, us: &String) {
        self.state.write().unwrap().noti_fail(us);
    }
    pub fn get_cm_cmd_sender(&self) -> Sender<Command> {
        return self.cmd_sender.clone();
    }
}
