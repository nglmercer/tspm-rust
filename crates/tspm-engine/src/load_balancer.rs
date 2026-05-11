use tspm_core::{InstanceInfo, LoadBalanceStrategy};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Load balancer trait
pub trait LoadBalancer: Send + Sync {
    fn next_instance(&self, instances: &[InstanceInfo]) -> Option<usize>;
}

/// Round-robin load balancer
pub struct RoundRobinBalancer {
    counter: AtomicUsize,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancer for RoundRobinBalancer {
    fn next_instance(&self, instances: &[InstanceInfo]) -> Option<usize> {
        let healthy: Vec<usize> = instances
            .iter()
            .enumerate()
            .filter(|(_, i)| i.healthy)
            .map(|(idx, _)| idx)
            .collect();

        if healthy.is_empty() {
            return None;
        }

        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        Some(healthy[count % healthy.len()])
    }
}

/// Create a load balancer from strategy
pub fn create_load_balancer(_strategy: LoadBalanceStrategy) -> Box<dyn LoadBalancer> {
    Box::new(RoundRobinBalancer::new())
}
