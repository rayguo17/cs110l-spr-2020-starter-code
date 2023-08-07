use rand::{Rng, SeedableRng};
use std::collections::HashMap;
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
    upstream_addresses: Vec<String>,
    upstream_statuses: HashMap<String, UpstreamStatus>,
}

pub struct UpstreamStatus {
    pub address: String,
    pub fail: bool,
}

impl ProxyState {
    pub fn new(
        ahci: usize,
        ahcp: String,
        mrpm: usize,
        ua: Vec<String>,
        us: HashMap<String, UpstreamStatus>,
    ) -> ProxyState {
        ProxyState {
            active_health_check_interval: ahci,
            active_health_check_path: ahcp,
            max_requests_per_minute: mrpm,
            upstream_addresses: ua,
            upstream_statuses: us,
        }
    }
    pub fn select_random_updastream(&self) -> Option<String> {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let upstream_idx = rng.gen_range(0, self.upstream_addresses.len());
        let upstream_ip = &self.upstream_addresses[upstream_idx];
        return Some(upstream_ip.to_string().clone());
    }
    pub fn noti_fail(&mut self, us: &String) {
        let length = self.upstream_addresses.len();
        for i in 0..length {
            if us.to_string() == self.upstream_addresses[i] {
                self.upstream_addresses.remove(i);
            }
        }
        match self.upstream_statuses.get_mut(us) {
            Some(ups) => {
                ups.fail = true;
            }
            None => {}
        }
    }
}
