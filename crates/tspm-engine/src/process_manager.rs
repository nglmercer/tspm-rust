use crate::cluster::ClusterManager;
use crate::managed_process::ManagedProcess;
use crate::registry::ProcessRegistry;
use tspm_core::{
    ClusterInfo, ProcessConfig, ProcessGroup, ProcessStatus, RestartReason,
    TspmConfig,
};

/// Central process manager that ties together registry, clustering, and lifecycle
pub struct ProcessManager {
    registry: ProcessRegistry<ManagedProcess>,
    cluster_manager: ClusterManager,
    config_dir: std::path::PathBuf,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            registry: ProcessRegistry::new(),
            cluster_manager: ClusterManager::new(),
            config_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        }
    }

    pub fn with_config_dir(config_dir: std::path::PathBuf) -> Self {
        Self {
            registry: ProcessRegistry::new(),
            cluster_manager: ClusterManager::new(),
            config_dir,
        }
    }

    pub fn set_config_dir(&mut self, dir: std::path::PathBuf) {
        self.config_dir = dir;
    }

    /// Add a process (and its instances) from config
    pub async fn add_process(&mut self, config: ProcessConfig) -> Result<(), String> {
        let instance_count = config.instances.max(1);
        let base_name = config.name.clone();
        let namespace = config.namespace.clone();
        let cluster_group = config.cluster_group.clone();

        // Create/get cluster
        let cluster = self.cluster_manager.get_or_create_cluster(&config);

        for i in 0..instance_count {
            let name = if i > 0 {
                format!("{base_name}-{i}")
            } else {
                base_name.clone()
            };

            // Remove existing process with same name
            if let Some(existing) = self.registry.get_mut(&name) {
                let _ = existing.stop().await;
            }
            self.registry.delete(&name);

            // Create new managed process
            let proc = ManagedProcess::new(config.clone(), i, self.config_dir.clone());

            // Register
            self.registry.add(
                name,
                proc,
                namespace.as_deref(),
                cluster_group.as_deref(),
            );

            // Add to cluster
            cluster.add_instance(i, config.instance_weight.unwrap_or(1));
        }

        Ok(())
    }

    /// Resolve a process name: tries exact match first, then matches by
    /// display name (basename of path) or config name. Returns the registry key.
    pub fn resolve_name(&self, name: &str) -> Option<String> {
        // 1. Exact match on registry key
        if self.registry.has(name) {
            return Some(name.to_string());
        }
        // 2. Match by display_name (basename) or config().name
        for proc in self.registry.get_all() {
            let display = proc.display_name();
            let cfg_name = &proc.config().name;
            // display_name is the registry key which could be a path
            let basename = std::path::Path::new(&display)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if basename == name || cfg_name == name || display == name {
                return Some(display);
            }
        }
        None
    }

    /// Get a managed process by name (supports fuzzy matching by basename)
    pub fn get_process(&self, name: &str) -> Option<&ManagedProcess> {
        self.resolve_name(name).and_then(|key| self.registry.get(&key))
    }

    /// Get a managed process mutably by name (supports fuzzy matching by basename)
    pub fn get_process_mut(&mut self, name: &str) -> Option<&mut ManagedProcess> {
        let key = self.resolve_name(name)?;
        self.registry.get_mut(&key)
    }

    /// Get all processes for a base name (including instances)
    pub fn get_processes_by_base_name(&self, base_name: &str) -> Vec<&ManagedProcess> {
        self.registry
            .get_all()
            .into_iter()
            .filter(|p| {
                let cfg_name = &p.config().name;
                let display = p.display_name();
                let basename = std::path::Path::new(&display)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                cfg_name == base_name || basename == base_name || display == base_name
            })
            .collect()
    }

    /// Start all managed processes
    pub async fn start_all(&mut self) -> Result<(), String> {
        // We need to collect names first because start() holds &mut
        let names: Vec<String> = self.registry.get_all().into_iter()
            .map(|p| p.display_name())
            .collect();

        for name in names {
            if let Some(proc) = self.registry.get_mut(&name) {
                proc.start().await?;
                let instance_id = proc.instance_id();
                let pid = proc.pid();
                let base = proc.config().name.clone();
                if let Some(cluster) = self.cluster_manager.get_cluster_mut(&base) {
                    cluster.update_instance_state(instance_id, proc.state(), pid);
                }
            }
        }

        Ok(())
    }

    /// Stop all managed processes
    pub async fn stop_all(&mut self) -> Result<(), String> {
        let names: Vec<String> = self.registry.get_all().into_iter()
            .map(|p| p.display_name())
            .collect();

        for name in names {
            if let Some(proc) = self.registry.get_mut(&name) {
                let _ = proc.stop().await;
            }
        }

        Ok(())
    }

    /// Start a specific process by name
    pub async fn start_process(&mut self, name: &str) -> Result<(), String> {
        let procs: Vec<String> = self
            .get_processes_by_base_name(name)
            .into_iter()
            .map(|p| p.display_name())
            .collect();

        if procs.is_empty() {
            if let Some(_) = self.registry.get(name) {
                if let Some(proc) = self.registry.get_mut(name) {
                    return proc.start().await;
                }
            }
            return Err(format!("Process not found: {name}"));
        }

        for name in procs {
            if let Some(proc) = self.registry.get_mut(&name) {
                proc.start().await?;
            }
        }

        Ok(())
    }

    /// Stop a specific process by name
    pub async fn stop_process(&mut self, name: &str) -> Result<(), String> {
        let procs: Vec<String> = self
            .get_processes_by_base_name(name)
            .into_iter()
            .map(|p| p.display_name())
            .collect();

        if procs.is_empty() {
            if let Some(proc) = self.registry.get_mut(name) {
                return proc.stop().await;
            }
            return Err(format!("Process not found: {name}"));
        }

        for name in procs {
            if let Some(proc) = self.registry.get_mut(&name) {
                proc.stop().await?;
            }
        }

        Ok(())
    }

    /// Restart a specific process by name
    pub async fn restart_process(&mut self, name: &str) -> Result<(), String> {
        let procs: Vec<String> = self
            .get_processes_by_base_name(name)
            .into_iter()
            .map(|p| p.display_name())
            .collect();

        if procs.is_empty() {
            if let Some(proc) = self.registry.get_mut(name) {
                return proc.restart(RestartReason::Manual).await;
            }
            return Err(format!("Process not found: {name}"));
        }

        for name in procs {
            if let Some(proc) = self.registry.get_mut(&name) {
                proc.restart(RestartReason::Manual).await?;
            }
        }

        Ok(())
    }

    /// Remove a process from management
    pub async fn remove_process(&mut self, name: &str) -> Result<(), String> {
        // Resolve the name (supports basename matching)
        let resolved = self.resolve_name(name).unwrap_or_else(|| name.to_string());
        let resolved_ref = resolved.as_str();

        // Stop instances first
        let procs: Vec<String> = self
            .get_processes_by_base_name(resolved_ref)
            .into_iter()
            .map(|p| p.display_name())
            .collect();

        let mut base_name = resolved.clone();
        if !procs.is_empty() {
            base_name = procs[0].split('-').next().unwrap_or(resolved_ref).to_string();
            for pname in &procs {
                if let Some(proc) = self.registry.get_mut(pname) {
                    proc.stop().await?;
                }
                self.registry.delete(pname);
            }
        } else if let Some(proc) = self.registry.get_mut(resolved_ref) {
            base_name = proc.config().name.clone();
            proc.stop().await?;
            self.registry.delete(resolved_ref);
        }

        // Remove from cluster
        self.cluster_manager.remove_cluster(&base_name);

        Ok(())
    }

    /// Get status of all managed processes
    pub fn get_statuses(&self) -> Vec<ProcessStatus> {
        self.registry.get_all().into_iter().map(|p| p.get_status()).collect()
    }

    /// Get cluster info for a process
    pub fn get_cluster_info(&self, name: &str) -> Option<ClusterInfo> {
        self.cluster_manager.get_cluster_info(name)
    }

    /// Get all namespaces
    pub fn get_namespaces(&self) -> Vec<String> {
        self.registry.get_namespaces()
    }

    /// Get all cluster groups
    pub fn get_cluster_groups(&self) -> Vec<String> {
        self.registry.get_cluster_groups()
    }

    /// Get process groups
    pub fn get_process_groups(&self) -> Vec<ProcessGroup> {
        let mut groups = Vec::new();
        let namespaces = self.registry.get_namespaces();

        for ns in &namespaces {
            let procs = self.registry.get_by_namespace(ns);
            let names: Vec<String> = procs.iter().map(|p| p.config().name.clone()).collect();
            let unique: std::collections::HashSet<_> = names.iter().collect();

            groups.push(ProcessGroup {
                name: ns.clone(),
                namespace: ns.clone(),
                process_count: unique.len(),
                total_instances: procs.len(),
                process_names: names,
            });
        }

        groups
    }

    /// Check if a process exists (supports fuzzy matching by basename)
    pub fn has_process(&self, name: &str) -> bool {
        self.resolve_name(name).is_some()
    }

    /// Scale instances for a process
    pub async fn scale_process(&mut self, base_name: &str, new_count: u32) -> Result<(), String> {
        let current = self.get_processes_by_base_name(base_name);
        let current_count = current.len() as u32;

        if new_count > current_count {
            let config = current.first().ok_or(format!("Process not found: {base_name}"))?.config().clone();
            let namespace = config.namespace.clone();
            let cluster_group = config.cluster_group.clone();
            let cluster = self.cluster_manager.get_or_create_cluster(&config);

            for i in current_count..new_count {
                let name = format!("{base_name}-{i}");
                let proc = ManagedProcess::new(config.clone(), i, self.config_dir.clone());

                self.registry.add(name.clone(), proc, namespace.as_deref(), cluster_group.as_deref());
                cluster.add_instance(i, config.instance_weight.unwrap_or(1));

                if let Some(proc) = self.registry.get_mut(&name) {
                    proc.start().await?;
                }
            }
        } else if new_count < current_count {
            for i in (new_count..current_count).rev() {
                let name = format!("{base_name}-{i}");
                if let Some(proc) = self.registry.get_mut(&name) {
                    proc.stop().await?;
                }
                self.registry.delete(&name);

                if let Some(cluster) = self.cluster_manager.get_cluster_mut(base_name) {
                    cluster.remove_instance(i);
                }
            }
        }

        Ok(())
    }

    /// Get the number of managed processes
    pub fn process_count(&self) -> usize {
        self.registry.size()
    }

    /// Get the number of clusters
    pub fn cluster_count(&self) -> usize {
        self.cluster_manager.size()
    }

    /// Load processes from a TSPM config
    pub async fn load_from_config(&mut self, config: &TspmConfig) -> Result<(), String> {
        for proc in &config.processes {
            self.add_process(proc.clone()).await?;
        }
        Ok(())
    }
}
