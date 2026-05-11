use std::collections::HashMap;

/// Registry for managed processes with namespace and cluster-group indexing
pub struct ProcessRegistry<T> {
    processes: HashMap<String, T>,
    namespaces: HashMap<String, Vec<String>>,
    cluster_groups: HashMap<String, Vec<String>>,
}

impl<T> ProcessRegistry<T> {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            namespaces: HashMap::new(),
            cluster_groups: HashMap::new(),
        }
    }

    pub fn add(
        &mut self,
        name: String,
        process: T,
        namespace: Option<&str>,
        cluster_group: Option<&str>,
    ) {
        if let Some(ns) = namespace {
            self.namespaces
                .entry(ns.to_string())
                .or_default()
                .push(name.clone());
        }
        if let Some(cg) = cluster_group {
            self.cluster_groups
                .entry(cg.to_string())
                .or_default()
                .push(name.clone());
        }
        self.processes.insert(name, process);
    }

    pub fn get(&self, name: &str) -> Option<&T> {
        self.processes.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        self.processes.get_mut(name)
    }

    pub fn delete(&mut self, name: &str) -> bool {
        // We don't clean up namespace/cluster-group entries for simplicity
        self.processes.remove(name).is_some()
    }

    pub fn get_all(&self) -> Vec<&T> {
        self.processes.values().collect()
    }

    pub fn get_by_namespace(&self, namespace: &str) -> Vec<&T> {
        let names = self.namespaces.get(namespace);
        match names {
            Some(names) => names
                .iter()
                .filter_map(|n| self.processes.get(n))
                .collect(),
            None => vec![],
        }
    }

    pub fn get_by_cluster_group(&self, group: &str) -> Vec<&T> {
        let names = self.cluster_groups.get(group);
        match names {
            Some(names) => names
                .iter()
                .filter_map(|n| self.processes.get(n))
                .collect(),
            None => vec![],
        }
    }

    pub fn get_namespaces(&self) -> Vec<String> {
        self.namespaces.keys().cloned().collect()
    }

    pub fn get_cluster_groups(&self) -> Vec<String> {
        self.cluster_groups.keys().cloned().collect()
    }

    pub fn has(&self, name: &str) -> bool {
        self.processes.contains_key(name)
    }

    pub fn size(&self) -> usize {
        self.processes.len()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.processes.values()
    }
}
