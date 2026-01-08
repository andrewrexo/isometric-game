use rand::Rng;

use super::prototype::EntityPrototype;
use crate::item::{GroundItem, ItemType};

/// Generate loot drops from a prototype's loot table
pub fn generate_loot_from_prototype(
    prototype: &EntityPrototype,
    x: f32,
    y: f32,
    killer_id: &str,
    current_time: u64,
    level: i32,
) -> Vec<GroundItem> {
    let mut drops = Vec::new();
    let mut rng = rand::thread_rng();
    let mut item_counter = 0u32;

    // Gold drop
    if prototype.rewards.gold_max > 0 {
        let gold_amount = rng.gen_range(
            prototype.rewards.gold_min..=prototype.rewards.gold_max
        ) * level; // Scale by level

        if gold_amount > 0 {
            let id = format!("item_{}_{}", current_time, item_counter);
            item_counter += 1;
            drops.push(GroundItem::new(
                &id,
                ItemType::Gold,
                x,
                y,
                gold_amount,
                Some(killer_id.to_string()),
                current_time,
            ));
        }
    }

    // Loot table drops
    for entry in &prototype.loot {
        if rng.gen::<f32>() < entry.drop_chance {
            let quantity = rng.gen_range(entry.quantity_min..=entry.quantity_max);
            if quantity > 0 {
                let id = format!("item_{}_{}", current_time, item_counter);
                item_counter += 1;

                // Map item_id to ItemType
                // TODO: Use ItemRegistry lookup when items are fully data-driven
                let item_type = match entry.item_id.as_str() {
                    "slime_core" => ItemType::SlimeCore,
                    "health_potion" => ItemType::HealthPotion,
                    "mana_potion" => ItemType::ManaPotion,
                    _ => continue, // Skip unknown items for now
                };

                drops.push(GroundItem::new(
                    &id,
                    item_type,
                    x + (item_counter as f32 * 0.3),
                    y + (item_counter as f32 * 0.3),
                    quantity,
                    Some(killer_id.to_string()),
                    current_time,
                ));
            }
        }
    }

    drops
}

/// Calculate exp reward for killing an entity
pub fn calculate_exp_reward(prototype: &EntityPrototype, level: i32) -> i32 {
    prototype.rewards.exp_base * level
}
