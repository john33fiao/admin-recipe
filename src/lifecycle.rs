use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Policy {
    pub ping_ms: u64,
    pub stale_ms: u64,
    pub grace_ms: u64,
    pub initial_ms: u64,
    pub resume_gap_ms: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            ping_ms: 15_000,
            stale_ms: 45_000,
            grace_ms: 120_000,
            initial_ms: 120_000,
            resume_gap_ms: 30_000,
        }
    }
}

pub struct Lifecycle {
    pub policy: Policy,
    connections: HashMap<u64, u64>,
    started: u64,
    last_tick: u64,
    connected_once: bool,
    empty_since: Option<u64>,
}

impl Lifecycle {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            connections: HashMap::new(),
            started: 0,
            last_tick: 0,
            connected_once: false,
            empty_since: None,
        }
    }
    pub fn connect(&mut self, id: u64, now: u64) {
        self.connected_once = true;
        self.empty_since = None;
        self.connections.insert(id, now);
    }
    pub fn pong(&mut self, id: u64, now: u64) {
        self.connect(id, now);
    }
    pub fn contains(&self, id: u64) -> bool {
        self.connections.contains_key(&id)
    }
    pub fn disconnect(&mut self, id: u64, now: u64) {
        self.connections.remove(&id);
        if self.connections.is_empty() {
            self.empty_since.get_or_insert(now);
        }
    }
    /// Both clocks are observed because suspend accounting differs across OSes.
    /// A long scheduler pause also earns a fresh reconnection opportunity.
    pub fn tick(&mut self, now: u64, wall_gap: u64) -> bool {
        let resumed = now.saturating_sub(self.last_tick) > self.policy.resume_gap_ms
            || wall_gap > self.policy.resume_gap_ms;
        self.last_tick = now;
        if resumed {
            self.started = now;
            self.empty_since = Some(now);
            for seen in self.connections.values_mut() {
                *seen = now;
            }
            return false;
        }
        self.connections
            .retain(|_, seen| now.saturating_sub(*seen) < self.policy.stale_ms);
        if !self.connections.is_empty() {
            self.empty_since = None;
            return false;
        }
        if !self.connected_once {
            return now.saturating_sub(self.started) >= self.policy.initial_ms;
        }
        let since = self.empty_since.get_or_insert(now);
        now.saturating_sub(*since) >= self.policy.grace_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tracker() -> Lifecycle {
        Lifecycle::new(Policy {
            ping_ms: 10,
            stale_ms: 45,
            grace_ms: 120,
            initial_ms: 120,
            resume_gap_ms: 300,
        })
    }
    #[test]
    fn no_browser_exits_after_initial_wait() {
        let mut l = tracker();
        assert!(!l.tick(119, 119));
        assert!(l.tick(120, 1));
    }
    #[test]
    fn another_live_tab_prevents_exit_then_last_tab_gets_grace() {
        let mut l = tracker();
        l.connect(1, 0);
        l.connect(2, 0);
        l.disconnect(1, 10);
        for t in [40, 80, 120, 160] {
            l.pong(2, t);
            assert!(!l.tick(t, 40));
        }
        l.disconnect(2, 170);
        assert!(!l.tick(289, 119));
        assert!(l.tick(290, 1));
    }
    #[test]
    fn refreshing_tab_cancels_pending_exit() {
        let mut l = tracker();
        l.connect(1, 0);
        l.disconnect(1, 10);
        assert!(!l.tick(100, 100));
        l.connect(2, 110);
        l.pong(2, 140);
        assert!(!l.tick(140, 40));
        l.disconnect(2, 150);
        assert!(!l.tick(269, 129));
        assert!(l.tick(270, 1));
    }
    #[test]
    fn missing_pong_expires_connection_before_shutdown_grace() {
        let mut l = tracker();
        l.connect(1, 0);
        assert!(!l.tick(45, 45));
        assert!(!l.tick(164, 119));
        assert!(l.tick(165, 1));
    }
    #[test]
    fn suspend_gap_gives_reconnect_opportunity_for_both_clocks() {
        for (now, wall_gap) in [(1000, 1000), (20, 1000)] {
            let mut l = tracker();
            l.connect(1, 0);
            l.disconnect(1, 10);
            assert!(!l.tick(now, wall_gap));
            l.connect(2, now + 1);
            l.pong(2, now + 40);
            assert!(!l.tick(now + 40, 40));
        }
    }
}
