use crate::CmdOptions;
use crossbeam_channel::{select, Receiver, Sender};
use rand::{Rng, SeedableRng};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use std::{thread, time};

use super::client_status::{ClientManager, Command};
/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
pub struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    // upstream_addresses: Vec<String>,
    // upstream_statuses: HashMap<String, UpstreamUnit>,
    _upstream_status: Arc<Mutex<UpstreamStatus>>,

    //should add a channel both sender and receiver
    health_success_receiver: crossbeam_channel::Receiver<String>,
    health_fail_receiver: crossbeam_channel::Receiver<String>,
    client_manager: ClientManager,
}

pub struct UpstreamUnit {
    pub address: String,
    pub fail: bool,
}

pub struct UpstreamStatus {
    ua: Vec<String>,
    us: HashMap<String, UpstreamUnit>,
}

impl UpstreamStatus {
    pub fn new(ua: Vec<String>, us: HashMap<String, UpstreamUnit>) -> UpstreamStatus {
        UpstreamStatus { ua: ua, us: us }
    }
    pub fn select_random_updastream(&self) -> Option<String> {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let upstream_idx = rng.gen_range(0, self.ua.len());
        let upstream_ip = &self.ua[upstream_idx];
        return Some(upstream_ip.to_string().clone());
    }
    pub fn noti_succ(&mut self, us: &String) {
        if self.ua.contains(us) {
            return;
        } else {
            self.ua.push(us.clone());
        }
        if self.us.contains_key(us) {
            match self.us.get_mut(us) {
                Some(ups) => {
                    ups.fail = false;
                }
                None => {}
            }
        }
    }
    pub fn noti_fail(&mut self, us: &String) {
        if self.ua.contains(us) {
            let length = self.ua.len();
            for i in 0..length {
                if us.to_string() == self.ua[i] {
                    self.ua.remove(i);
                }
            }
        } else {
            return;
        }

        match self.us.get_mut(us) {
            Some(ups) => {
                ups.fail = true;
            }
            None => {}
        }
    }
    pub fn get_up_addrs(&self) -> Vec<String> {
        return self.ua.clone();
    }
}

impl ProxyState {
    pub fn new(
        ahci: usize,
        ahcp: String,
        mrpm: usize,
        // ua: Vec<String>,
        // us: HashMap<String, UpstreamUnit>,
        _us: Arc<Mutex<UpstreamStatus>>,
        hsr: crossbeam_channel::Receiver<String>,
        hfr: crossbeam_channel::Receiver<String>,
    ) -> ProxyState {
        ProxyState {
            active_health_check_interval: ahci,
            active_health_check_path: ahcp,
            max_requests_per_minute: mrpm,
            // upstream_addresses: ua,
            // upstream_statuses: us,
            health_success_receiver: hsr,
            health_fail_receiver: hfr,
            _upstream_status: _us,
            client_manager: ClientManager::new(),
        }
    }
    pub fn main_routine_invoker(
        &self,
        sr: &Receiver<String>,
        fr: &Receiver<String>,
    ) -> JoinHandle<()> {
        let ups = self._upstream_status.clone();
        let sr = sr.clone();
        let fr = fr.clone();
        let thread = thread::spawn(move || {
            Self::main_routine_worker(ups, sr, fr);
        });
        return thread;
    }
    pub fn get_cm_cmd_sender(&self) -> Sender<Command> {
        return self.client_manager.cmd_send.clone();
    }
    pub fn client_manager_main_routine_invoker(&self) -> JoinHandle<()> {
        return self
            .client_manager
            .inner_routine_invoker(self.max_requests_per_minute);
    }
    pub fn valid_printer(ua: &Arc<Mutex<UpstreamStatus>>) {
        let uas = ua.lock().unwrap();
        let que = uas.get_up_addrs();
        print!("valid addr:");
        for addr in que.iter() {
            print!(" {}", addr);
        }
        println!("");
    }
    pub fn main_routine_worker(
        ua: Arc<Mutex<UpstreamStatus>>,
        sr: Receiver<String>,
        fr: Receiver<String>,
    ) {
        // match different receiver, and then do modification,
        loop {
            Self::valid_printer(&ua);
            select! {
                //only one channel would be better for order gurantee.
                recv(sr)->msg => {
                    match msg {
                        Ok(m)=>{
                            //log::debug!("in main routine_worker: {}",m);
                            //
                            let mut ua = ua.lock().unwrap();
                            ua.noti_succ(&m);
                        },
                        Err(e)=>{
                            log::debug!("in main routine_worker: {}",e)
                        }
                    }
                },
                recv(fr)->msg =>{
                    match msg {
                        Ok(m)=>{
                            let mut ua = ua.lock().unwrap();
                            ua.noti_fail(&m);
                            //log::debug!("in main routine_worker: {}",m)
                        },
                        Err(e)=>{
                            log::debug!("in main routine_worker: {}",e)
                        }
                    }
                }
            }
            //log::debug!("Health check on going!");
            thread::sleep(time::Duration::from_secs(1));
        }
    }
    pub fn get_option(&self) -> CmdOptions {
        let ups = self._upstream_status.lock().unwrap().get_up_addrs();
        return CmdOptions {
            bind: "".to_string(),
            upstream: ups,
            active_health_check_interval: self.active_health_check_interval.clone(),
            active_health_check_path: self.active_health_check_path.clone(),
            max_requests_per_minute: self.max_requests_per_minute.clone(),
        };
    }
    pub fn select_random_updastream(&self) -> Option<String> {
        return self
            ._upstream_status
            .lock()
            .unwrap()
            .select_random_updastream();
    }
    pub fn noti_fail(&self, us: &String) {
        self._upstream_status.lock().unwrap().noti_fail(us);
    }
}
