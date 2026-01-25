use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn, error};
use crate::interior::InteriorMapDef;

pub struct InteriorRegistry {
    interiors: HashMap<String, InteriorMapDef>,
}

impl InteriorRegistry {
    pub fn load_from_directory(dir: &str) -> Result<Self, String> {
        let mut interiors = HashMap::new();
        let path = Path::new(dir);

        if !path.exists() {
            warn!("Interiors directory {} does not exist, creating empty registry", dir);
            return Ok(Self { interiors });
        }

        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read interiors directory: {}", e))?;

        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "json") {
                match InteriorMapDef::load_from_file(file_path.to_str().unwrap()) {
                    Ok(interior) => {
                        info!("Loaded interior map: {} ({})", interior.id, interior.name);
                        interiors.insert(interior.id.clone(), interior);
                    }
                    Err(e) => {
                        error!("Failed to load interior {:?}: {}", file_path, e);
                    }
                }
            }
        }

        info!("Loaded {} interior maps", interiors.len());
        Ok(Self { interiors })
    }

    pub fn get(&self, id: &str) -> Option<&InteriorMapDef> {
        self.interiors.get(id)
    }

    pub fn list_ids(&self) -> Vec<&String> {
        self.interiors.keys().collect()
    }
}
