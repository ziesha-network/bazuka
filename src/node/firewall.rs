use super::*;

pub struct Firewall {
    request_count_limit_per_minute: usize,
    request_count_last_reset: Timestamp,
    request_count: HashMap<IpAddr, usize>,

    traffic_limit_per_15m: u64,
    traffic_last_reset: Timestamp,
    traffic: HashMap<IpAddr, u64>,
}

impl Firewall {
    pub fn new(request_count_limit_per_minute: usize, traffic_limit_per_15m: u64) -> Self {
        Self {
            request_count_limit_per_minute,
            traffic_limit_per_15m,
            request_count_last_reset: 0,
            request_count: HashMap::new(),
            traffic_last_reset: 0,
            traffic: HashMap::new(),
        }
    }
    pub fn refresh(&mut self, now: u32) {
        if now - self.request_count_last_reset > 60 {
            self.request_count.clear();
            self.request_count_last_reset = now;
        }

        if now - self.traffic_last_reset > 900 {
            self.traffic.clear();
            self.traffic_last_reset = now;
        }
    }
    pub fn add_traffic(&mut self, ip: IpAddr, amount: u64) {
        *self.traffic.entry(ip).or_insert(0) += amount;
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
