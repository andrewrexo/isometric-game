use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::interior::InstanceType;
use crate::npc::Npc;

/// Tracks an active instance
pub struct Instance {
    pub id: String,
    pub map_id: String,
    pub instance_type: InstanceType,
    pub owner_id: Option<String>,  // For private instances
    pub players: RwLock<HashSet<String>>,
    /// NPCs spawned in this instance
    pub npcs: RwLock<HashMap<String, Npc>>,
    /// Whether NPCs have been spawned for this instance
    pub npcs_spawned: RwLock<bool>,
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

    /// Spawn NPCs from entity definitions (call once when instance is created)
    pub async fn spawn_npcs(
        &self,
        entities: &[crate::interior::InteriorEntitySpawn],
        entity_registry: &crate::entity::registry::EntityRegistry,
    ) {
        let mut spawned = self.npcs_spawned.write().await;
        if *spawned {
            return; // Already spawned
        }

        let mut npcs = self.npcs.write().await;
        for (i, spawn) in entities.iter().enumerate() {
            let npc_id = spawn.unique_id.clone()
                .unwrap_or_else(|| format!("{}_{}", self.id, i));

            if let Some(prototype) = entity_registry.get(&spawn.entity_id) {
                info!("Spawning {} at ({}, {}) in instance {}",
                    spawn.entity_id, spawn.x, spawn.y, self.id);
                let npc = Npc::from_prototype(
                    &npc_id,
                    &spawn.entity_id,
                    prototype,
                    spawn.x,
                    spawn.y,
                    spawn.level,
                );
                npcs.insert(npc_id, npc);
            } else {
                tracing::warn!("Prototype '{}' not found for instance {}", spawn.entity_id, self.id);
            }
        }

        *spawned = true;
        info!("Spawned {} NPCs in instance {}", npcs.len(), self.id);
    }

    /// Get all NPCs in this instance
    pub async fn get_npcs(&self) -> HashMap<String, Npc> {
        self.npcs.read().await.clone()
    }

    /// Get NPC updates for sending to clients
    pub async fn get_npc_updates(&self) -> Vec<crate::npc::NpcUpdate> {
        let npcs = self.npcs.read().await;
        npcs.values().map(|npc| crate::npc::NpcUpdate::from(npc)).collect()
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
    pub fn get_or_create_public(&self, map_id: &str) -> (Arc<Instance>, bool) {
        if let Some(instance) = self.public_instances.get(map_id) {
            return (instance.clone(), false);
        }

        let instance_id = format!("pub_{}", map_id);
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_id.to_string(),
            instance_type: InstanceType::Public,
            owner_id: None,
            players: RwLock::new(HashSet::new()),
            npcs: RwLock::new(HashMap::new()),
            npcs_spawned: RwLock::new(false),
        });

        self.public_instances.insert(map_id.to_string(), instance.clone());
        info!("Created public instance: {}", instance_id);
        (instance, true)
    }

    /// Get or create a private instance for a player
    pub fn get_or_create_private(&self, map_id: &str, owner_id: &str) -> (Arc<Instance>, bool) {
        let key = (owner_id.to_string(), map_id.to_string());

        if let Some(instance) = self.private_instances.get(&key) {
            return (instance.clone(), false);
        }

        let instance_id = format!("priv_{}_{}", map_id, Uuid::new_v4());
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_id.to_string(),
            instance_type: InstanceType::Private,
            owner_id: Some(owner_id.to_string()),
            players: RwLock::new(HashSet::new()),
            npcs: RwLock::new(HashMap::new()),
            npcs_spawned: RwLock::new(false),
        });

        self.private_instances.insert(key, instance.clone());
        info!("Created private instance: {} for owner {}", instance_id, owner_id);
        (instance, true)
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
