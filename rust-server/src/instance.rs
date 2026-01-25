use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::interior::InstanceType;

/// Tracks an active instance
pub struct Instance {
    pub id: String,
    pub map_id: String,
    pub instance_type: InstanceType,
    pub owner_id: Option<String>,  // For private instances
    pub players: RwLock<HashSet<String>>,
}

impl Instance {
    pub async fn player_count(&self) -> usize {
        self.players.read().await.len()
    }

    pub async fn add_player(&self, player_id: &str) {
        self.players.write().await.insert(player_id.to_string());
    }

    pub async fn remove_player(&self, player_id: &str) -> usize {
        let mut players = self.players.write().await;
        players.remove(player_id);
        players.len()
    }

    pub async fn has_player(&self, player_id: &str) -> bool {
        self.players.read().await.contains(player_id)
    }
}

/// Manages all active instances
pub struct InstanceManager {
    /// Public instances: one per map_id
    pub public_instances: DashMap<String, Arc<Instance>>,
    /// Private instances: keyed by (owner_player_id, map_id)
    pub private_instances: DashMap<(String, String), Arc<Instance>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            public_instances: DashMap::new(),
            private_instances: DashMap::new(),
        }
    }

    /// Get or create a public instance for a map
    pub fn get_or_create_public(&self, map_id: &str) -> Arc<Instance> {
        if let Some(instance) = self.public_instances.get(map_id) {
            return instance.clone();
        }

        let instance_id = format!("pub_{}", map_id);
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_id.to_string(),
            instance_type: InstanceType::Public,
            owner_id: None,
            players: RwLock::new(HashSet::new()),
        });

        self.public_instances.insert(map_id.to_string(), instance.clone());
        info!("Created public instance: {}", instance_id);
        instance
    }

    /// Get or create a private instance for a player
    pub fn get_or_create_private(&self, map_id: &str, owner_id: &str) -> Arc<Instance> {
        let key = (owner_id.to_string(), map_id.to_string());

        if let Some(instance) = self.private_instances.get(&key) {
            return instance.clone();
        }

        let instance_id = format!("priv_{}_{}", map_id, Uuid::new_v4());
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_id.to_string(),
            instance_type: InstanceType::Private,
            owner_id: Some(owner_id.to_string()),
            players: RwLock::new(HashSet::new()),
        });

        self.private_instances.insert(key, instance.clone());
        info!("Created private instance: {} for owner {}", instance_id, owner_id);
        instance
    }

    /// Remove a private instance (called when empty)
    pub fn remove_private(&self, owner_id: &str, map_id: &str) {
        let key = (owner_id.to_string(), map_id.to_string());
        if self.private_instances.remove(&key).is_some() {
            info!("Removed private instance for {} / {}", owner_id, map_id);
        }
    }

    /// Find which instance a player is in by checking all instances
    pub async fn find_player_instance(&self, player_id: &str) -> Option<Arc<Instance>> {
        for entry in self.public_instances.iter() {
            if entry.value().has_player(player_id).await {
                return Some(entry.value().clone());
            }
        }
        for entry in self.private_instances.iter() {
            if entry.value().has_player(player_id).await {
                return Some(entry.value().clone());
            }
        }
        None
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}
