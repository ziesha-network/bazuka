use super::*;

pub struct Firewall {
    request_count_limit_per_minute: usize,
    request_count_last_reset: Timestamp,
    request_count: HashMap<IpAddr, usize>,

    traffic_limit_per_minute: u64,
    traffic_last_reset: Timestamp,
    traffic: HashMap<IpAddr, u64>,
}

impl Firewall {
    pub fn new(request_count_limit_per_minute: usize, traffic_limit_per_minute: u64) -> Self {
        Self {
            request_count_limit_per_minute,
            traffic_limit_per_minute,
            request_count_last_reset: 0,
            request_count: HashMap::new(),
            traffic_last_reset: 0,
            traffic: HashMap::new(),
        }
    }
    pub fn refresh(&mut self, now: u32) {
        if now.saturating_sub(self.request_count_last_reset) > 60 {
            self.request_count.clear();
            self.request_count_last_reset = now;
        }

        if now.saturating_sub(self.traffic_last_reset) > 60 {
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

        if self.traffic.get(&client.ip()).cloned().unwrap_or(0) > self.traffic_limit_per_minute {
            return false;
        }

        let cnt = self.request_count.entry(client.ip()).or_insert(0);
        if *cnt >= self.request_count_limit_per_minute {
            return false;
        }

        *cnt += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_limit() {
        let mut firewall = Firewall::new(10, 1000);
        firewall.refresh(1234);
        let client: SocketAddr = "123.234.56.78:12345".parse().unwrap();
        for _ in 0..10 {
            assert!(firewall.incoming_permitted(client));
        }
        // Do not allow after 10 reqs
        assert!(!firewall.incoming_permitted(client));

        // Not allowed before timer reset
        firewall.refresh(1235);
        assert!(!firewall.incoming_permitted(client));
        firewall.refresh(1240);
        assert!(!firewall.incoming_permitted(client));

        // Go back in time
        firewall.refresh(1230);
        assert!(!firewall.incoming_permitted(client));

        firewall.refresh(1293);
        assert!(!firewall.incoming_permitted(client));
        firewall.refresh(1294);
        assert!(!firewall.incoming_permitted(client));

        // Reset! Allowed again :)
        firewall.refresh(1295);
        assert!(firewall.incoming_permitted(client));
    }
}
