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

/// A map object placed from object layer (trees, rocks, decorations)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteriorMapObject {
    pub gid: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A wall placed on a tile edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorWall {
    pub gid: u32,
    pub x: i32,
    pub y: i32,
    pub edge: String,
}

/// Entity spawn point in an interior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteriorEntitySpawn {
    pub entity_id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_level")]
    pub level: i32,
    #[serde(default)]
    pub unique_id: Option<String>,
    #[serde(default)]
    pub facing: Option<String>,
    #[serde(default = "default_true")]
    pub respawn: bool,
}

fn default_level() -> i32 { 1 }
fn default_true() -> bool { true }

/// Definition of an interior map (loaded from JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorMapDef {
    pub id: String,
    pub name: String,
    pub instance_type: InstanceType,
    pub size: InteriorSize,
    pub spawn_points: HashMap<String, SpawnPoint>,
    pub portals: Vec<InteriorPortal>,
    // Map data fields
    #[serde(default)]
    pub layers: InteriorLayers,
    #[serde(default)]
    pub collision: String,  // Base64 encoded or empty
    #[serde(default)]
    pub entities: Vec<InteriorEntitySpawn>,
    #[serde(default, rename = "mapObjects")]
    pub map_objects: Vec<InteriorMapObject>,
    #[serde(default)]
    pub walls: Vec<InteriorWall>,
}

/// Layer data for interior maps
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InteriorLayers {
    #[serde(default)]
    pub ground: Vec<u32>,
    #[serde(default)]
    pub objects: Vec<u32>,
    #[serde(default)]
    pub overhead: Vec<u32>,
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

pub use crate::interior_registry::InteriorRegistry;
