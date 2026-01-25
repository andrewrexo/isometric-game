use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of instance this interior creates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceType {
    Public,
    Private,
}

/// A spawn point inside an interior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
}

/// A portal/exit inside an interior that leads elsewhere
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorPortal {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,
    pub target_x: f32,
    pub target_y: f32,
    pub target_spawn: Option<String>,
}

/// Definition of an interior map (loaded from JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorMapDef {
    pub id: String,
    pub name: String,
    pub instance_type: InstanceType,
    pub size: InteriorSize,
    pub spawn_points: HashMap<String, SpawnPoint>,
    pub portals: Vec<InteriorPortal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorSize {
    pub width: u32,
    pub height: u32,
}

impl InteriorMapDef {
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read interior file {}: {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse interior JSON {}: {}", path, e))
    }

    pub fn get_spawn_point(&self, name: &str) -> Option<&SpawnPoint> {
        self.spawn_points.get(name)
    }

    pub fn get_portal_at(&self, x: i32, y: i32) -> Option<&InteriorPortal> {
        self.portals.iter().find(|p| {
            x >= p.x && x < p.x + p.width && y >= p.y && y < p.y + p.height
        })
    }
}
