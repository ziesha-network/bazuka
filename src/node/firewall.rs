use super::*;

pub struct Firewall {
    request_count_limit_per_minute: usize,
    traffic_limit_per_15m: u64,
    unresponsive_count_limit_per_15m: usize,
    bad_ips: HashMap<IpAddr, Timestamp>,
    unresponsive_ips: HashMap<IpAddr, Timestamp>,
    unresponsive_count_last_reset: Timestamp,
    unresponsive_count: HashMap<IpAddr, usize>,
    request_count_last_reset: Timestamp,
    request_count: HashMap<IpAddr, usize>,
    traffic_last_reset: Timestamp,
    traffic: HashMap<IpAddr, u64>,
}

impl Firewall {
    pub fn new(
        request_count_limit_per_minute: usize,
        traffic_limit_per_15m: u64,
        unresponsive_count_limit_per_15m: usize,
    ) -> Self {
        Self {
            request_count_limit_per_minute,
            traffic_limit_per_15m,
            unresponsive_count_limit_per_15m,
            bad_ips: HashMap::new(),
            unresponsive_ips: HashMap::new(),
            request_count_last_reset: 0,
            unresponsive_count_last_reset: 0,
            request_count: HashMap::new(),
            traffic_last_reset: 0,
            traffic: HashMap::new(),
            unresponsive_count: HashMap::new(),
        }
    }
    pub fn refresh(&mut self) {
        for ip in self.bad_ips.clone().into_keys() {
            if !self.is_ip_bad(ip) {
                self.bad_ips.remove(&ip);
            }
        }
        for ip in self.unresponsive_ips.clone().into_keys() {
            if !self.is_ip_unresponsive(ip) {
                self.unresponsive_ips.remove(&ip);
            }
        }

        let ts = local_timestamp();

        if ts - self.unresponsive_count_last_reset > 900 {
            self.unresponsive_count.clear();
            self.unresponsive_count_last_reset = ts;
        }

        if ts - self.request_count_last_reset > 60 {
            self.request_count.clear();
            self.request_count_last_reset = ts;
        }

        if ts - self.traffic_last_reset > 900 {
            self.traffic.clear();
            self.traffic_last_reset = ts;
        }
    }
    pub fn add_traffic(&mut self, ip: IpAddr, amount: u64) {
        *self.traffic.entry(ip).or_insert(0) += amount;
    }
    pub fn punish_bad(&mut self, ip: IpAddr, secs: u32) {
        let now = local_timestamp();
        let ts = self.bad_ips.entry(ip).or_insert(0);
        *ts = std::cmp::max(*ts, now) + secs;
    }
    pub fn is_peer_dead(&self, peer: PeerAddress) -> bool {
        let ip = peer.0.ip();

        // Loopback is never dead
        if ip.is_loopback() {
            return false;
        }

        if let Some(cnt) = self.unresponsive_count.get(&ip) {
            if *cnt > self.unresponsive_count_limit_per_15m {
                return true;
            }
        }
        false
    }
    pub fn punish_unresponsive(&mut self, ip: IpAddr, secs: u32, max_punish: u32) {
        let now = local_timestamp();
        let ts = self.unresponsive_ips.entry(ip).or_insert(0);
        let cnt = self.unresponsive_count.entry(ip).or_insert(0);
        *cnt += 1;
        *ts = std::cmp::min(std::cmp::max(*ts, now) + secs, now + max_punish);
    }
    fn is_ip_bad(&self, ip: IpAddr) -> bool {
        // Loopback is never bad
        if ip.is_loopback() {
            return false;
        }

        if let Some(punished_until) = self.bad_ips.get(&ip) {
            if local_timestamp() < *punished_until {
                return true;
            }
        }
        false
    }
    fn is_ip_unresponsive(&self, ip: IpAddr) -> bool {
        // Loopback is never unresponsive
        if ip.is_loopback() {
            return false;
        }

        if let Some(punished_until) = self.unresponsive_ips.get(&ip) {
            if local_timestamp() < *punished_until {
                return true;
            }
        }
        false
    }
    pub fn outgoing_permitted(&self, peer: PeerAddress) -> bool {
        let ip = peer.0.ip();

        // Outgoing to loopback is always permitted.
        if ip.is_loopback() {
            return true;
        }

        if self.is_ip_bad(ip) || self.is_ip_unresponsive(ip) {
            return false;
        }

        true
    }
    pub fn incoming_permitted(&mut self, client: SocketAddr) -> bool {
        // Incoming from loopback is always permitted
        if client.ip().is_loopback() {
            return true;
        }

        if self.is_ip_bad(client.ip()) {
            return false;
        }

        if self.traffic.get(&client.ip()).cloned().unwrap_or(0) > self.traffic_limit_per_15m {
            return false;
        }

        let cnt = self.request_count.entry(client.ip()).or_insert(0);
        if *cnt > self.request_count_limit_per_minute {
            return false;
        }

        *cnt += 1;
        true
    }
}
