use std::collections::HashMap;
use tspm_core::{
    ProcessConfig, InstanceInfo, ClusterInfo, LoadBalanceStrategy,
    ProcessState,
};

/// Lightweight process cluster for load balancing instances
#[derive(Debug, Clone)]
pub struct ProcessCluster {
    name: String,
    strategy: LoadBalanceStrategy,
    instances: HashMap<u32, InstanceInfo>,
}

impl ProcessCluster {
    pub fn new(name: String, strategy: LoadBalanceStrategy) -> Self {
        Self {
            name,
            strategy,
            instances: HashMap::new(),
        }
    }

    pub fn add_instance(&mut self, id: u32, weight: u32) {
        self.instances.insert(
            id,
            InstanceInfo {
                id,
                name: format!("{}-{}", self.name, if id > 0 { id.to_string() } else { self.name.clone() }),
                connections: 0,
                cpu: 0.0,
                memory: 0,
                weight,
                healthy: true,
                state: Some(ProcessState::Stopped),
                pid: None,
                started_at: None,
            },
        );
    }

    pub fn remove_instance(&mut self, id: u32) {
        self.instances.remove(&id);
    }

    pub fn get_instances(&self) -> Vec<&InstanceInfo> {
        self.instances.values().collect()
    }

    pub fn get_strategy(&self) -> LoadBalanceStrategy {
        self.strategy
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    pub fn get_next_instance(&self) -> Option<&InstanceInfo> {
        let healthy: Vec<_> = self.instances.values().filter(|i| i.healthy).collect();
        if healthy.is_empty() {
            self.instances.values().next()
        } else {
            healthy.first().copied()
        }
    }

    pub fn update_instance_state(&mut self, id: u32, state: ProcessState, pid: Option<u32>) {
        if let Some(info) = self.instances.get_mut(&id) {
            info.state = Some(state);
            info.pid = pid;
        }
    }
}

/// Manages all clusters
#[derive(Default)]
pub struct ClusterManager {
    clusters: HashMap<String, ProcessCluster>,
}

impl ClusterManager {
    pub fn new() -> Self {
        Self {
            clusters: HashMap::new(),
        }
    }

    pub fn get_or_create_cluster(&mut self, config: &ProcessConfig) -> &mut ProcessCluster {
        self.clusters.entry(config.name.clone()).or_insert_with(|| {
            ProcessCluster::new(
                config.name.clone(),
                config.lb_strategy.unwrap_or(LoadBalanceStrategy::RoundRobin),
            )
        })
    }

    pub fn get_cluster(&self, name: &str) -> Option<&ProcessCluster> {
        self.clusters.get(name)
    }

    pub fn get_cluster_mut(&mut self, name: &str) -> Option<&mut ProcessCluster> {
        self.clusters.get_mut(name)
    }

    pub fn remove_cluster(&mut self, name: &str) {
        self.clusters.remove(name);
    }

    pub fn get_all_clusters(&self) -> &HashMap<String, ProcessCluster> {
        &self.clusters
    }

    pub fn size(&self) -> usize {
        self.clusters.len()
    }

    pub fn get_cluster_info(&self, name: &str) -> Option<ClusterInfo> {
        let cluster = self.clusters.get(name)?;
        let instances: Vec<InstanceInfo> = cluster.instances.values().cloned().collect();
        let running = instances.iter().filter(|i| i.state == Some(ProcessState::Running)).count();
        let healthy = instances.iter().filter(|i| i.healthy).count();

        Some(ClusterInfo {
            name: cluster.name.clone(),
            total_instances: instances.len(),
            running_instances: running,
            healthy_instances: healthy,
            strategy: cluster.strategy,
            instances,
        })
    }
}
