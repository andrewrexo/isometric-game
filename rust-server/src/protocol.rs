use serde::{Deserialize, Serialize};

use crate::game::PlayerUpdate;
use crate::npc::NpcUpdate;

// ============================================================================
// Client -> Server Messages
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "move")]
    Move { dx: f32, dy: f32 },

    #[serde(rename = "face")]
    Face { direction: u8 },

    #[serde(rename = "chat")]
    Chat { text: String },

    #[serde(rename = "attack")]
    Attack,

    #[serde(rename = "target")]
    Target { entity_id: String },

    #[serde(rename = "pickup")]
    Pickup { item_id: String },

    #[serde(rename = "useItem")]
    UseItem { slot_index: u8 },

    #[serde(rename = "auth")]
    Auth { username: String, password: String },

    #[serde(rename = "register")]
    Register { username: String, password: String },

    #[serde(rename = "requestChunk")]
    RequestChunk { chunk_x: i32, chunk_y: i32 },

    /// Interact with an NPC (quest giver, merchant, etc.)
    #[serde(rename = "interact")]
    Interact { npc_id: String },

    /// Player selected a dialogue choice
    #[serde(rename = "dialogueChoice")]
    DialogueChoiceMsg { quest_id: String, choice_id: String },

    /// Player accepts a quest
    #[serde(rename = "acceptQuest")]
    AcceptQuest { quest_id: String },

    /// Player abandons a quest
    #[serde(rename = "abandonQuest")]
    AbandonQuest { quest_id: String },

    /// Player requests to craft an item
    #[serde(rename = "craft")]
    Craft { recipe_id: String },

    /// Equip item from inventory slot
    #[serde(rename = "equip")]
    Equip { slot_index: u8 },

    /// Unequip item from equipment slot
    #[serde(rename = "unequip")]
    Unequip { slot_type: String },

    /// Drop item from inventory slot
    #[serde(rename = "dropItem")]
    DropItem { slot_index: u8, quantity: u32 },

    /// Drop gold to the ground
    #[serde(rename = "dropGold")]
    DropGold { amount: i32 },

    /// Swap two inventory slots
    #[serde(rename = "swapSlots")]
    SwapSlots { from_slot: u8, to_slot: u8 },

    /// Buy item from shop
    #[serde(rename = "shopBuy")]
    ShopBuy { npc_id: String, item_id: String, quantity: i32 },

    /// Sell item to shop
    #[serde(rename = "shopSell")]
    ShopSell { npc_id: String, item_id: String, quantity: i32 },
}

// ============================================================================
// Server -> Client Messages
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ServerMessage {
    Welcome {
        player_id: String,
    },
    PlayerJoined {
        id: String,
        name: String,
        x: i32,
        y: i32,
        gender: String,
        skin: String,
        hair_style: Option<i32>,
        hair_color: Option<i32>,
    },
    PlayerLeft {
        id: String,
    },
    StateSync {
        tick: u64,
        players: Vec<PlayerUpdate>,
        npcs: Vec<NpcUpdate>,
    },
    ChatMessage {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "senderName")]
        sender_name: String,
        text: String,
        timestamp: u64,
    },
    TargetChanged {
        player_id: String,
        target_id: Option<String>,
    },
    DamageEvent {
        source_id: String,
        target_id: String,
        damage: i32,
        target_hp: i32,
        target_x: f32,
        target_y: f32,
        projectile: Option<String>,
    },
    AttackResult {
        success: bool,
        reason: Option<String>,
    },
    NpcDied {
        id: String,
        killer_id: String,
    },
    NpcRespawned {
        id: String,
        x: i32,
        y: i32,
    },
    PlayerDied {
        id: String,
        killer_id: String,
    },
    PlayerRespawned {
        id: String,
        x: i32,
        y: i32,
        hp: i32,
    },
    SkillXp {
        player_id: String,
        skill: String,
        xp_gained: i64,
        total_xp: i64,
        level: i32,
    },
    SkillLevelUp {
        player_id: String,
        skill: String,
        new_level: i32,
    },
    ItemDropped {
        id: String,
        item_id: String,
        x: f32,
        y: f32,
        quantity: i32,
    },
    ItemPickedUp {
        item_id: String,
        player_id: String,
    },
    ItemDespawned {
        item_id: String,
    },
    ItemQuantityUpdated {
        id: String,
        quantity: i32,
    },
    InventoryUpdate {
        player_id: String,
        slots: Vec<crate::item::InventorySlotUpdate>,
        gold: i32,
    },
    ItemUsed {
        player_id: String,
        slot: u8,
        item_id: String,
        effect: String, // e.g., "heal:30"
    },
    // Quest-related messages
    QuestAccepted {
        quest_id: String,
        quest_name: String,
        objectives: Vec<QuestObjectiveData>,
    },
    QuestObjectiveProgress {
        quest_id: String,
        objective_id: String,
        current: i32,
        target: i32,
    },
    QuestCompleted {
        quest_id: String,
        quest_name: String,
        rewards_exp: i32,
        rewards_gold: i32,
    },
    ShowDialogue {
        quest_id: String,
        npc_id: String,
        speaker: String,
        text: String,
        choices: Vec<DialogueChoice>,
    },
    Error {
        code: u32,
        message: String,
    },
    ChunkData {
        chunk_x: i32,
        chunk_y: i32,
        layers: Vec<ChunkLayerData>,
        collision: Vec<u8>, // Packed collision bits
        objects: Vec<ChunkObjectData>, // Map objects (trees, rocks, etc.)
    },
    ChunkNotFound {
        chunk_x: i32,
        chunk_y: i32,
    },
    /// Sent on connect: all entity definitions for client-side registry
    EntityDefinitions {
        entities: Vec<ClientEntityDef>,
    },
    /// Sent on connect: all item definitions for client-side registry
    ItemDefinitions {
        items: Vec<ClientItemDef>,
    },
    /// Tell client to close the dialogue UI
    DialogueClosed,
    /// Sent on connect: all recipe definitions for client-side registry
    RecipeDefinitions {
        recipes: Vec<ClientRecipeDef>,
    },
    /// Result of a crafting attempt
    CraftResult {
        success: bool,
        recipe_id: String,
        error: Option<String>,
        items_gained: Vec<RecipeResult>,
    },
    /// Tell client to open the shop/crafting UI for a merchant NPC
    ShopOpen {
        npc_id: String,
    },
    /// Send shop data to client
    ShopData {
        npc_id: String,
        shop: ShopData,
    },
    /// Result of a shop buy/sell action
    ShopResult {
        success: bool,
        action: String,
        item_id: String,
        quantity: i32,
        gold_change: i32,
        error: Option<String>,
    },
    /// Broadcast shop stock update to nearby players
    ShopStockUpdate {
        npc_id: String,
        item_id: String,
        new_quantity: i32,
    },
    /// Broadcast equipment change to all players
    EquipmentUpdate {
        player_id: String,
        equipped_head: Option<String>,
        equipped_body: Option<String>,
        equipped_weapon: Option<String>,
        equipped_back: Option<String>,
        equipped_feet: Option<String>,
        equipped_ring: Option<String>,
        equipped_gloves: Option<String>,
        equipped_necklace: Option<String>,
        equipped_belt: Option<String>,
    },
    /// Result of equip/unequip action sent to the acting player
    EquipResult {
        success: bool,
        slot_type: String,
        item_id: Option<String>,
        error: Option<String>,
    },
    /// Server-wide announcement from admin
    Announcement {
        text: String,
    },
}

/// Layer data for chunk transmission
#[derive(Debug, Clone, Serialize)]
pub struct ChunkLayerData {
    pub layer_type: u8, // 0=Ground, 1=Objects, 2=Overhead
    pub tiles: Vec<u32>,
}

/// Map object data for chunk transmission (trees, rocks, decorations)
#[derive(Debug, Clone, Serialize)]
pub struct ChunkObjectData {
    pub gid: u32,      // Global tile ID from objects.tsx
    pub tile_x: i32,   // World tile X coordinate
    pub tile_y: i32,   // World tile Y coordinate
    pub width: u32,    // Sprite width in pixels
    pub height: u32,   // Sprite height in pixels
}

/// Entity definition for client-side registry
#[derive(Debug, Clone, Serialize)]
pub struct ClientEntityDef {
    pub id: String,
    pub display_name: String,
    pub sprite: String,
    pub animation_type: String, // "blob", "humanoid", "quadruped", "flying"
    pub max_hp: i32,
}

/// Item definition for client-side registry
#[derive(Debug, Clone, Serialize)]
pub struct ClientItemDef {
    pub id: String,
    pub display_name: String,
    pub sprite: String,
    pub category: String, // "consumable", "material", "equipment", "quest"
    pub max_stack: i32,
    pub description: String,
    pub base_price: i32,
    pub sellable: bool,
    // Equipment-specific fields (None for non-equipment items)
    pub equipment_slot: Option<String>,
    pub attack_level_required: Option<i32>,
    pub defence_level_required: Option<i32>,
    pub attack_bonus: Option<i32>,
    pub strength_bonus: Option<i32>,
    pub defence_bonus: Option<i32>,
    pub weapon_type: Option<String>,
    pub range: Option<i32>,
}

/// A dialogue choice for branching dialogue
#[derive(Debug, Clone, Serialize)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
}

/// Quest objective data for QuestAccepted message
#[derive(Debug, Clone, Serialize)]
pub struct QuestObjectiveData {
    pub id: String,
    pub description: String,
    pub current: i32,
    pub target: i32,
    pub completed: bool,
}

/// Recipe ingredient for client sync
#[derive(Debug, Clone, Serialize)]
pub struct RecipeIngredient {
    pub item_id: String,
    pub item_name: String,
    pub count: i32,
}

/// Recipe result for client sync
#[derive(Debug, Clone, Serialize)]
pub struct RecipeResult {
    pub item_id: String,
    pub item_name: String,
    pub count: i32,
}

/// Recipe definition for client-side registry
#[derive(Debug, Clone, Serialize)]
pub struct ClientRecipeDef {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub level_required: i32,
    pub ingredients: Vec<RecipeIngredient>,
    pub results: Vec<RecipeResult>,
}

/// Shop data for client synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopData {
    pub shop_id: String,
    pub display_name: String,
    pub buy_multiplier: f32,
    pub sell_multiplier: f32,
    pub stock: Vec<ShopStockItemData>,
}

/// Shop stock item data for client synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItemData {
    pub item_id: String,
    pub quantity: i32,
    pub price: i32,
}

impl ServerMessage {
    pub fn msg_type(&self) -> &'static str {
        match self {
            ServerMessage::Welcome { .. } => "welcome",
            ServerMessage::PlayerJoined { .. } => "playerJoined",
            ServerMessage::PlayerLeft { .. } => "playerLeft",
            ServerMessage::StateSync { .. } => "stateSync",
            ServerMessage::ChatMessage { .. } => "chatMessage",
            ServerMessage::TargetChanged { .. } => "targetChanged",
            ServerMessage::DamageEvent { .. } => "damageEvent",
            ServerMessage::AttackResult { .. } => "attackResult",
            ServerMessage::NpcDied { .. } => "npcDied",
            ServerMessage::NpcRespawned { .. } => "npcRespawned",
            ServerMessage::PlayerDied { .. } => "playerDied",
            ServerMessage::PlayerRespawned { .. } => "playerRespawned",
            ServerMessage::SkillXp { .. } => "skillXp",
            ServerMessage::SkillLevelUp { .. } => "skillLevelUp",
            ServerMessage::ItemDropped { .. } => "itemDropped",
            ServerMessage::ItemPickedUp { .. } => "itemPickedUp",
            ServerMessage::ItemDespawned { .. } => "itemDespawned",
            ServerMessage::ItemQuantityUpdated { .. } => "itemQuantityUpdated",
            ServerMessage::InventoryUpdate { .. } => "inventoryUpdate",
            ServerMessage::ItemUsed { .. } => "itemUsed",
            ServerMessage::QuestAccepted { .. } => "questAccepted",
            ServerMessage::QuestObjectiveProgress { .. } => "questObjectiveProgress",
            ServerMessage::QuestCompleted { .. } => "questCompleted",
            ServerMessage::ShowDialogue { .. } => "showDialogue",
            ServerMessage::Error { .. } => "error",
            ServerMessage::ChunkData { .. } => "chunkData",
            ServerMessage::ChunkNotFound { .. } => "chunkNotFound",
            ServerMessage::EntityDefinitions { .. } => "entityDefinitions",
            ServerMessage::ItemDefinitions { .. } => "itemDefinitions",
            ServerMessage::DialogueClosed => "dialogueClosed",
            ServerMessage::RecipeDefinitions { .. } => "recipeDefinitions",
            ServerMessage::CraftResult { .. } => "craftResult",
            ServerMessage::ShopOpen { .. } => "shopOpen",
            ServerMessage::ShopData { .. } => "shopData",
            ServerMessage::ShopResult { .. } => "shopResult",
            ServerMessage::ShopStockUpdate { .. } => "shopStockUpdate",
            ServerMessage::EquipmentUpdate { .. } => "equipmentUpdate",
            ServerMessage::EquipResult { .. } => "equipResult",
            ServerMessage::Announcement { .. } => "announcement",
        }
    }
}

// ============================================================================
// Encoding/Decoding
// ============================================================================

/// Encode a server message to MessagePack format
/// Format: [13, "msg_type", {data}] (matching Colyseus ROOM_DATA protocol)
pub fn encode_server_message(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    use rmpv::Value;

    let msg_type = msg.msg_type();

    // Convert message to rmpv::Value
    let data = match msg {
        ServerMessage::Welcome { player_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::PlayerJoined { id, name, x, y, gender, skin, hair_style, hair_color } => {
            let mut map = Vec::new();
            map.push((Value::String("id".into()), Value::String(id.clone().into())));
            map.push((
                Value::String("name".into()),
                Value::String(name.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            map.push((Value::String("gender".into()), Value::String(gender.clone().into())));
            map.push((Value::String("skin".into()), Value::String(skin.clone().into())));
            map.push((
                Value::String("hair_style".into()),
                match hair_style {
                    Some(style) => Value::Integer((*style as i64).into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("hair_color".into()),
                match hair_color {
                    Some(color) => Value::Integer((*color as i64).into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::PlayerLeft { id } => {
            let mut map = Vec::new();
            map.push((Value::String("id".into()), Value::String(id.clone().into())));
            Value::Map(map)
        }
        ServerMessage::StateSync { tick, players, npcs } => {
            let mut map = Vec::new();
            map.push((Value::String("tick".into()), Value::Integer((*tick).into())));

            let player_values: Vec<Value> = players
                .iter()
                .map(|p| {
                    let mut pmap = Vec::new();
                    pmap.push((
                        Value::String("id".into()),
                        Value::String(p.id.clone().into()),
                    ));
                    pmap.push((Value::String("x".into()), Value::Integer((p.x as i64).into())));
                    pmap.push((Value::String("y".into()), Value::Integer((p.y as i64).into())));
                    pmap.push((
                        Value::String("direction".into()),
                        Value::Integer((p.direction as i64).into()),
                    ));
                    // Include velocity for client-side prediction
                    pmap.push((Value::String("velX".into()), Value::Integer((p.vel_x as i64).into())));
                    pmap.push((Value::String("velY".into()), Value::Integer((p.vel_y as i64).into())));
                    pmap.push((Value::String("hp".into()), Value::Integer((p.hp as i64).into())));
                    pmap.push((Value::String("maxHp".into()), Value::Integer((p.max_hp as i64).into())));
                    pmap.push((Value::String("combatLevel".into()), Value::Integer((p.combat_level as i64).into())));
                    // Individual skill levels
                    pmap.push((Value::String("hitpointsLevel".into()), Value::Integer((p.hitpoints_level as i64).into())));
                    pmap.push((Value::String("combatSkillLevel".into()), Value::Integer((p.combat_skill_level as i64).into())));
                    pmap.push((Value::String("gold".into()), Value::Integer((p.gold as i64).into())));
                    pmap.push((Value::String("gender".into()), Value::String(p.gender.clone().into())));
                    pmap.push((Value::String("skin".into()), Value::String(p.skin.clone().into())));
                    pmap.push((
                        Value::String("hair_style".into()),
                        match p.hair_style {
                            Some(style) => Value::Integer((style as i64).into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("hair_color".into()),
                        match p.hair_color {
                            Some(color) => Value::Integer((color as i64).into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_head".into()),
                        match &p.equipped_head {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_body".into()),
                        match &p.equipped_body {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_weapon".into()),
                        match &p.equipped_weapon {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_back".into()),
                        match &p.equipped_back {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_feet".into()),
                        match &p.equipped_feet {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_ring".into()),
                        match &p.equipped_ring {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_gloves".into()),
                        match &p.equipped_gloves {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_necklace".into()),
                        match &p.equipped_necklace {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("equipped_belt".into()),
                        match &p.equipped_belt {
                            Some(item_id) => Value::String(item_id.clone().into()),
                            None => Value::Nil,
                        },
                    ));
                    pmap.push((
                        Value::String("is_admin".into()),
                        Value::Boolean(p.is_admin),
                    ));
                    Value::Map(pmap)
                })
                .collect();
            map.push((Value::String("players".into()), Value::Array(player_values)));

            let npc_values: Vec<Value> = npcs
                .iter()
                .map(|n| {
                    let mut nmap = Vec::new();
                    nmap.push((
                        Value::String("id".into()),
                        Value::String(n.id.clone().into()),
                    ));
                    nmap.push((Value::String("entity_type".into()), Value::String(n.entity_type.clone().into())));
                    nmap.push((Value::String("display_name".into()), Value::String(n.display_name.clone().into())));
                    nmap.push((Value::String("x".into()), Value::Integer((n.x as i64).into())));
                    nmap.push((Value::String("y".into()), Value::Integer((n.y as i64).into())));
                    nmap.push((Value::String("direction".into()), Value::Integer((n.direction as i64).into())));
                    nmap.push((Value::String("hp".into()), Value::Integer((n.hp as i64).into())));
                    nmap.push((Value::String("max_hp".into()), Value::Integer((n.max_hp as i64).into())));
                    nmap.push((Value::String("level".into()), Value::Integer((n.level as i64).into())));
                    nmap.push((Value::String("state".into()), Value::Integer((n.state as i64).into())));
                    nmap.push((Value::String("hostile".into()), Value::Boolean(n.hostile)));
                    nmap.push((Value::String("is_quest_giver".into()), Value::Boolean(n.is_quest_giver)));
                    nmap.push((Value::String("is_merchant".into()), Value::Boolean(n.is_merchant)));
                    nmap.push((Value::String("move_speed".into()), Value::F32(n.move_speed)));
                    nmap.push((Value::String("just_attacked".into()), Value::Boolean(n.just_attacked)));
                    Value::Map(nmap)
                })
                .collect();
            map.push((Value::String("npcs".into()), Value::Array(npc_values)));

            Value::Map(map)
        }
        ServerMessage::ChatMessage {
            sender_id,
            sender_name,
            text,
            timestamp,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("senderId".into()),
                Value::String(sender_id.clone().into()),
            ));
            map.push((
                Value::String("senderName".into()),
                Value::String(sender_name.clone().into()),
            ));
            map.push((
                Value::String("text".into()),
                Value::String(text.clone().into()),
            ));
            map.push((
                Value::String("timestamp".into()),
                Value::Integer((*timestamp).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::TargetChanged {
            player_id,
            target_id,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("target_id".into()),
                match target_id {
                    Some(id) => Value::String(id.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::DamageEvent {
            source_id,
            target_id,
            damage,
            target_hp,
            target_x,
            target_y,
            projectile,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("source_id".into()),
                Value::String(source_id.clone().into()),
            ));
            map.push((
                Value::String("target_id".into()),
                Value::String(target_id.clone().into()),
            ));
            map.push((
                Value::String("damage".into()),
                Value::Integer((*damage as i64).into()),
            ));
            map.push((
                Value::String("target_hp".into()),
                Value::Integer((*target_hp as i64).into()),
            ));
            map.push((
                Value::String("target_x".into()),
                Value::F64(*target_x as f64),
            ));
            map.push((
                Value::String("target_y".into()),
                Value::F64(*target_y as f64),
            ));
            map.push((
                Value::String("projectile".into()),
                match projectile {
                    Some(p) => Value::String(p.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::AttackResult { success, reason } => {
            let mut map = Vec::new();
            map.push((
                Value::String("success".into()),
                Value::Boolean(*success),
            ));
            map.push((
                Value::String("reason".into()),
                match reason {
                    Some(r) => Value::String(r.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::NpcDied { id, killer_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("killer_id".into()),
                Value::String(killer_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::NpcRespawned { id, x, y } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            Value::Map(map)
        }
        ServerMessage::PlayerDied { id, killer_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("killer_id".into()),
                Value::String(killer_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::PlayerRespawned { id, x, y, hp } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            map.push((Value::String("hp".into()), Value::Integer((*hp as i64).into())));
            Value::Map(map)
        }
        ServerMessage::SkillXp {
            player_id,
            skill,
            xp_gained,
            total_xp,
            level,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("skill".into()),
                Value::String(skill.clone().into()),
            ));
            map.push((
                Value::String("xp_gained".into()),
                Value::Integer((*xp_gained).into()),
            ));
            map.push((
                Value::String("total_xp".into()),
                Value::Integer((*total_xp).into()),
            ));
            map.push((
                Value::String("level".into()),
                Value::Integer((*level as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::SkillLevelUp {
            player_id,
            skill,
            new_level,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("skill".into()),
                Value::String(skill.clone().into()),
            ));
            map.push((
                Value::String("new_level".into()),
                Value::Integer((*new_level as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemDropped {
            id,
            item_id,
            x,
            y,
            quantity,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("item_id".into()),
                Value::String(item_id.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::F64(*x as f64)));
            map.push((Value::String("y".into()), Value::F64(*y as f64)));
            map.push((
                Value::String("quantity".into()),
                Value::Integer((*quantity as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemPickedUp { item_id, player_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("item_id".into()),
                Value::String(item_id.clone().into()),
            ));
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemDespawned { item_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("item_id".into()),
                Value::String(item_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemQuantityUpdated { id, quantity } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("quantity".into()),
                Value::Integer((*quantity as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::InventoryUpdate { player_id, slots, gold } => {
            let mut map = Vec::new();
            map.push((Value::String("player_id".into()), Value::String(player_id.clone().into())));

            let slot_values: Vec<Value> = slots.iter().map(|s| {
                let mut smap = Vec::new();
                smap.push((Value::String("slot".into()), Value::Integer((s.slot as i64).into())));
                smap.push((Value::String("item_id".into()), Value::String(s.item_id.clone().into())));
                smap.push((Value::String("quantity".into()), Value::Integer((s.quantity as i64).into())));
                Value::Map(smap)
            }).collect();

            map.push((Value::String("slots".into()), Value::Array(slot_values)));
            map.push((Value::String("gold".into()), Value::Integer((*gold as i64).into())));
            Value::Map(map)
        }
        ServerMessage::ItemUsed { player_id, slot, item_id, effect } => {
            let mut map = Vec::new();
            map.push((Value::String("player_id".into()), Value::String(player_id.clone().into())));
            map.push((Value::String("slot".into()), Value::Integer((*slot as i64).into())));
            map.push((Value::String("item_id".into()), Value::String(item_id.clone().into())));
            map.push((Value::String("effect".into()), Value::String(effect.clone().into())));
            Value::Map(map)
        }
        ServerMessage::QuestAccepted { quest_id, quest_name, objectives } => {
            let mut map = Vec::new();
            map.push((Value::String("quest_id".into()), Value::String(quest_id.clone().into())));
            map.push((Value::String("quest_name".into()), Value::String(quest_name.clone().into())));

            let obj_values: Vec<Value> = objectives.iter().map(|obj| {
                let mut omap = Vec::new();
                omap.push((Value::String("id".into()), Value::String(obj.id.clone().into())));
                omap.push((Value::String("description".into()), Value::String(obj.description.clone().into())));
                omap.push((Value::String("current".into()), Value::Integer((obj.current as i64).into())));
                omap.push((Value::String("target".into()), Value::Integer((obj.target as i64).into())));
                omap.push((Value::String("completed".into()), Value::Boolean(obj.completed)));
                Value::Map(omap)
            }).collect();
            map.push((Value::String("objectives".into()), Value::Array(obj_values)));

            Value::Map(map)
        }
        ServerMessage::QuestObjectiveProgress { quest_id, objective_id, current, target } => {
            let mut map = Vec::new();
            map.push((Value::String("quest_id".into()), Value::String(quest_id.clone().into())));
            map.push((Value::String("objective_id".into()), Value::String(objective_id.clone().into())));
            map.push((Value::String("current".into()), Value::Integer((*current as i64).into())));
            map.push((Value::String("target".into()), Value::Integer((*target as i64).into())));
            Value::Map(map)
        }
        ServerMessage::QuestCompleted { quest_id, quest_name, rewards_exp, rewards_gold } => {
            let mut map = Vec::new();
            map.push((Value::String("quest_id".into()), Value::String(quest_id.clone().into())));
            map.push((Value::String("quest_name".into()), Value::String(quest_name.clone().into())));
            map.push((Value::String("rewards_exp".into()), Value::Integer((*rewards_exp as i64).into())));
            map.push((Value::String("rewards_gold".into()), Value::Integer((*rewards_gold as i64).into())));
            Value::Map(map)
        }
        ServerMessage::ShowDialogue { quest_id, npc_id, speaker, text, choices } => {
            let mut map = Vec::new();
            map.push((Value::String("quest_id".into()), Value::String(quest_id.clone().into())));
            map.push((Value::String("npc_id".into()), Value::String(npc_id.clone().into())));
            map.push((Value::String("speaker".into()), Value::String(speaker.clone().into())));
            map.push((Value::String("text".into()), Value::String(text.clone().into())));

            let choice_values: Vec<Value> = choices.iter().map(|c| {
                let mut cmap = Vec::new();
                cmap.push((Value::String("id".into()), Value::String(c.id.clone().into())));
                cmap.push((Value::String("text".into()), Value::String(c.text.clone().into())));
                Value::Map(cmap)
            }).collect();
            map.push((Value::String("choices".into()), Value::Array(choice_values)));

            Value::Map(map)
        }
        ServerMessage::Error { code, message } => {
            let mut map = Vec::new();
            map.push((
                Value::String("code".into()),
                Value::Integer((*code as i64).into()),
            ));
            map.push((
                Value::String("message".into()),
                Value::String(message.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ChunkData {
            chunk_x,
            chunk_y,
            layers,
            collision,
            objects,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("chunkX".into()),
                Value::Integer((*chunk_x as i64).into()),
            ));
            map.push((
                Value::String("chunkY".into()),
                Value::Integer((*chunk_y as i64).into()),
            ));

            // Encode layers
            let layer_values: Vec<Value> = layers
                .iter()
                .map(|l| {
                    let mut lmap = Vec::new();
                    lmap.push((
                        Value::String("layerType".into()),
                        Value::Integer((l.layer_type as i64).into()),
                    ));
                    let tiles: Vec<Value> = l
                        .tiles
                        .iter()
                        .map(|&t| Value::Integer((t as i64).into()))
                        .collect();
                    lmap.push((Value::String("tiles".into()), Value::Array(tiles)));
                    Value::Map(lmap)
                })
                .collect();
            map.push((Value::String("layers".into()), Value::Array(layer_values)));

            // Encode collision as binary
            let collision_bytes: Vec<Value> = collision
                .iter()
                .map(|&b| Value::Integer((b as i64).into()))
                .collect();
            map.push((
                Value::String("collision".into()),
                Value::Array(collision_bytes),
            ));

            // Encode map objects
            let object_values: Vec<Value> = objects
                .iter()
                .map(|o| {
                    let mut omap = Vec::new();
                    omap.push((
                        Value::String("gid".into()),
                        Value::Integer((o.gid as i64).into()),
                    ));
                    omap.push((
                        Value::String("tileX".into()),
                        Value::Integer((o.tile_x as i64).into()),
                    ));
                    omap.push((
                        Value::String("tileY".into()),
                        Value::Integer((o.tile_y as i64).into()),
                    ));
                    omap.push((
                        Value::String("width".into()),
                        Value::Integer((o.width as i64).into()),
                    ));
                    omap.push((
                        Value::String("height".into()),
                        Value::Integer((o.height as i64).into()),
                    ));
                    Value::Map(omap)
                })
                .collect();
            map.push((Value::String("objects".into()), Value::Array(object_values)));

            Value::Map(map)
        }
        ServerMessage::ChunkNotFound { chunk_x, chunk_y } => {
            let mut map = Vec::new();
            map.push((
                Value::String("chunkX".into()),
                Value::Integer((*chunk_x as i64).into()),
            ));
            map.push((
                Value::String("chunkY".into()),
                Value::Integer((*chunk_y as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::EntityDefinitions { entities } => {
            let mut map = Vec::new();
            let entity_values: Vec<Value> = entities
                .iter()
                .map(|e| {
                    let mut emap = Vec::new();
                    emap.push((Value::String("id".into()), Value::String(e.id.clone().into())));
                    emap.push((Value::String("displayName".into()), Value::String(e.display_name.clone().into())));
                    emap.push((Value::String("sprite".into()), Value::String(e.sprite.clone().into())));
                    emap.push((Value::String("animationType".into()), Value::String(e.animation_type.clone().into())));
                    emap.push((Value::String("maxHp".into()), Value::Integer((e.max_hp as i64).into())));
                    Value::Map(emap)
                })
                .collect();
            map.push((Value::String("entities".into()), Value::Array(entity_values)));
            Value::Map(map)
        }
        ServerMessage::ItemDefinitions { items } => {
            let mut map = Vec::new();
            let item_values: Vec<Value> = items
                .iter()
                .map(|i| {
                    let mut imap = Vec::new();
                    imap.push((Value::String("id".into()), Value::String(i.id.clone().into())));
                    imap.push((Value::String("displayName".into()), Value::String(i.display_name.clone().into())));
                    imap.push((Value::String("sprite".into()), Value::String(i.sprite.clone().into())));
                    imap.push((Value::String("category".into()), Value::String(i.category.clone().into())));
                    imap.push((Value::String("maxStack".into()), Value::Integer((i.max_stack as i64).into())));
                    imap.push((Value::String("description".into()), Value::String(i.description.clone().into())));
                    imap.push((Value::String("basePrice".into()), Value::Integer((i.base_price as i64).into())));
                    imap.push((Value::String("sellable".into()), Value::Boolean(i.sellable)));
                    // Add equipment fields if present
                    if let Some(ref slot) = i.equipment_slot {
                        imap.push((Value::String("equipment_slot".into()), Value::String(slot.clone().into())));
                    }
                    if let Some(level) = i.attack_level_required {
                        imap.push((Value::String("attack_level_required".into()), Value::Integer((level as i64).into())));
                    }
                    if let Some(level) = i.defence_level_required {
                        imap.push((Value::String("defence_level_required".into()), Value::Integer((level as i64).into())));
                    }
                    if let Some(bonus) = i.attack_bonus {
                        imap.push((Value::String("attack_bonus".into()), Value::Integer((bonus as i64).into())));
                    }
                    if let Some(bonus) = i.strength_bonus {
                        imap.push((Value::String("strength_bonus".into()), Value::Integer((bonus as i64).into())));
                    }
                    if let Some(def) = i.defence_bonus {
                        imap.push((Value::String("defence_bonus".into()), Value::Integer((def as i64).into())));
                    }
                    if let Some(ref wtype) = i.weapon_type {
                        imap.push((Value::String("weapon_type".into()), Value::String(wtype.clone().into())));
                    }
                    if let Some(r) = i.range {
                        imap.push((Value::String("range".into()), Value::Integer((r as i64).into())));
                    }
                    Value::Map(imap)
                })
                .collect();
            map.push((Value::String("items".into()), Value::Array(item_values)));
            Value::Map(map)
        }
        ServerMessage::DialogueClosed => {
            // Empty map - just the message type signals closure
            Value::Map(Vec::new())
        }
        ServerMessage::RecipeDefinitions { recipes } => {
            let mut map = Vec::new();
            let recipe_values: Vec<Value> = recipes
                .iter()
                .map(|r| {
                    let mut rmap = Vec::new();
                    rmap.push((Value::String("id".into()), Value::String(r.id.clone().into())));
                    rmap.push((Value::String("display_name".into()), Value::String(r.display_name.clone().into())));
                    rmap.push((Value::String("description".into()), Value::String(r.description.clone().into())));
                    rmap.push((Value::String("category".into()), Value::String(r.category.clone().into())));
                    rmap.push((Value::String("level_required".into()), Value::Integer((r.level_required as i64).into())));

                    let ingredient_values: Vec<Value> = r.ingredients.iter().map(|i| {
                        let mut imap = Vec::new();
                        imap.push((Value::String("item_id".into()), Value::String(i.item_id.clone().into())));
                        imap.push((Value::String("item_name".into()), Value::String(i.item_name.clone().into())));
                        imap.push((Value::String("count".into()), Value::Integer((i.count as i64).into())));
                        Value::Map(imap)
                    }).collect();
                    rmap.push((Value::String("ingredients".into()), Value::Array(ingredient_values)));

                    let result_values: Vec<Value> = r.results.iter().map(|res| {
                        let mut resmap = Vec::new();
                        resmap.push((Value::String("item_id".into()), Value::String(res.item_id.clone().into())));
                        resmap.push((Value::String("item_name".into()), Value::String(res.item_name.clone().into())));
                        resmap.push((Value::String("count".into()), Value::Integer((res.count as i64).into())));
                        Value::Map(resmap)
                    }).collect();
                    rmap.push((Value::String("results".into()), Value::Array(result_values)));

                    Value::Map(rmap)
                })
                .collect();
            map.push((Value::String("recipes".into()), Value::Array(recipe_values)));
            Value::Map(map)
        }
        ServerMessage::CraftResult { success, recipe_id, error, items_gained } => {
            let mut map = Vec::new();
            map.push((Value::String("success".into()), Value::Boolean(*success)));
            map.push((Value::String("recipeId".into()), Value::String(recipe_id.clone().into())));
            map.push((
                Value::String("error".into()),
                match error {
                    Some(e) => Value::String(e.clone().into()),
                    None => Value::Nil,
                },
            ));

            let item_values: Vec<Value> = items_gained.iter().map(|item| {
                let mut imap = Vec::new();
                imap.push((Value::String("itemId".into()), Value::String(item.item_id.clone().into())));
                imap.push((Value::String("count".into()), Value::Integer((item.count as i64).into())));
                Value::Map(imap)
            }).collect();
            map.push((Value::String("itemsGained".into()), Value::Array(item_values)));

            Value::Map(map)
        }
        ServerMessage::ShopOpen { npc_id } => {
            let mut map = Vec::new();
            map.push((Value::String("npc_id".into()), Value::String(npc_id.clone().into())));
            Value::Map(map)
        }
        ServerMessage::ShopData { npc_id, shop } => {
            let mut map = Vec::new();
            map.push((Value::String("npcId".into()), Value::String(npc_id.clone().into())));

            let mut shop_map = Vec::new();
            shop_map.push((Value::String("shopId".into()), Value::String(shop.shop_id.clone().into())));
            shop_map.push((Value::String("displayName".into()), Value::String(shop.display_name.clone().into())));
            shop_map.push((Value::String("buyMultiplier".into()), Value::F64(shop.buy_multiplier as f64)));
            shop_map.push((Value::String("sellMultiplier".into()), Value::F64(shop.sell_multiplier as f64)));

            let stock_values: Vec<Value> = shop.stock.iter().map(|s| {
                let mut smap = Vec::new();
                smap.push((Value::String("itemId".into()), Value::String(s.item_id.clone().into())));
                smap.push((Value::String("quantity".into()), Value::Integer((s.quantity as i64).into())));
                smap.push((Value::String("price".into()), Value::Integer((s.price as i64).into())));
                Value::Map(smap)
            }).collect();
            shop_map.push((Value::String("stock".into()), Value::Array(stock_values)));

            map.push((Value::String("shop".into()), Value::Map(shop_map)));
            Value::Map(map)
        }
        ServerMessage::ShopResult { success, action, item_id, quantity, gold_change, error } => {
            let mut map = Vec::new();
            map.push((Value::String("success".into()), Value::Boolean(*success)));
            map.push((Value::String("action".into()), Value::String(action.clone().into())));
            map.push((Value::String("itemId".into()), Value::String(item_id.clone().into())));
            map.push((Value::String("quantity".into()), Value::Integer((*quantity as i64).into())));
            map.push((Value::String("goldChange".into()), Value::Integer((*gold_change as i64).into())));
            map.push((
                Value::String("error".into()),
                match error {
                    Some(e) => Value::String(e.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::ShopStockUpdate { npc_id, item_id, new_quantity } => {
            let mut map = Vec::new();
            map.push((Value::String("npcId".into()), Value::String(npc_id.clone().into())));
            map.push((Value::String("itemId".into()), Value::String(item_id.clone().into())));
            map.push((Value::String("newQuantity".into()), Value::Integer((*new_quantity as i64).into())));
            Value::Map(map)
        }
        ServerMessage::EquipmentUpdate { player_id, equipped_head, equipped_body, equipped_weapon, equipped_back, equipped_feet, equipped_ring, equipped_gloves, equipped_necklace, equipped_belt } => {
            let mut map = Vec::new();
            map.push((Value::String("player_id".into()), Value::String(player_id.clone().into())));
            map.push((
                Value::String("equipped_head".into()),
                match equipped_head {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_body".into()),
                match equipped_body {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_weapon".into()),
                match equipped_weapon {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_back".into()),
                match equipped_back {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_feet".into()),
                match equipped_feet {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_ring".into()),
                match equipped_ring {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_gloves".into()),
                match equipped_gloves {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_necklace".into()),
                match equipped_necklace {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("equipped_belt".into()),
                match equipped_belt {
                    Some(item_id) => Value::String(item_id.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::EquipResult { success, slot_type, item_id, error } => {
            let mut map = Vec::new();
            map.push((Value::String("success".into()), Value::Boolean(*success)));
            map.push((Value::String("slot_type".into()), Value::String(slot_type.clone().into())));
            map.push((
                Value::String("item_id".into()),
                match item_id {
                    Some(id) => Value::String(id.clone().into()),
                    None => Value::Nil,
                },
            ));
            map.push((
                Value::String("error".into()),
                match error {
                    Some(e) => Value::String(e.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::Announcement { text } => {
            let mut map = Vec::new();
            map.push((Value::String("text".into()), Value::String(text.clone().into())));
            Value::Map(map)
        }
    };

    // Encode as [13, "msg_type", data] - matching Colyseus ROOM_DATA format
    let array = Value::Array(vec![
        Value::Integer(13.into()), // Protocol.RoomData
        Value::String(msg_type.into()),
        data,
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &array)
        .map_err(|e| format!("Failed to encode message: {}", e))?;

    Ok(buf)
}

/// Decode a client message from MessagePack format
/// Expected format: [13, "msg_type", {data}]
pub fn decode_client_message(data: &[u8]) -> Result<ClientMessage, String> {
    use rmpv::Value;
    use std::io::Cursor;

    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode MessagePack: {}", e))?;

    let array = value
        .as_array()
        .ok_or("Expected array")?;

    if array.len() < 2 {
        return Err("Array too short".to_string());
    }

    let protocol = array[0]
        .as_u64()
        .ok_or("Protocol code must be integer")? as u8;

    if protocol != 13 {
        return Err(format!("Unexpected protocol code: {}", protocol));
    }

    let msg_type = array[1]
        .as_str()
        .ok_or("Message type must be string")?;

    let msg_data = if array.len() > 2 {
        &array[2]
    } else {
        &Value::Nil
    };

    match msg_type {
        "move" => {
            let dx = extract_f32(msg_data, "dx").unwrap_or(0.0);
            let dy = extract_f32(msg_data, "dy").unwrap_or(0.0);
            Ok(ClientMessage::Move { dx, dy })
        }
        "face" => {
            let direction = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("direction")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            Ok(ClientMessage::Face { direction })
        }
        "chat" => {
            let text = extract_string(msg_data, "text").unwrap_or_default();
            Ok(ClientMessage::Chat { text })
        }
        "attack" => Ok(ClientMessage::Attack),
        "target" => {
            let entity_id = extract_string(msg_data, "entity_id").unwrap_or_default();
            Ok(ClientMessage::Target { entity_id })
        }
        "pickup" => {
            let item_id = extract_string(msg_data, "item_id").unwrap_or_default();
            Ok(ClientMessage::Pickup { item_id })
        }
        "useItem" => {
            let slot_index = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("slot_index")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            Ok(ClientMessage::UseItem { slot_index })
        }
        "auth" => {
            let username = extract_string(msg_data, "username").unwrap_or_default();
            let password = extract_string(msg_data, "password").unwrap_or_default();
            Ok(ClientMessage::Auth { username, password })
        }
        "register" => {
            let username = extract_string(msg_data, "username").unwrap_or_default();
            let password = extract_string(msg_data, "password").unwrap_or_default();
            Ok(ClientMessage::Register { username, password })
        }
        "requestChunk" => {
            let chunk_x = extract_i32(msg_data, "chunkX").unwrap_or(0);
            let chunk_y = extract_i32(msg_data, "chunkY").unwrap_or(0);
            Ok(ClientMessage::RequestChunk { chunk_x, chunk_y })
        }
        "interact" => {
            let npc_id = extract_string(msg_data, "npc_id").unwrap_or_default();
            Ok(ClientMessage::Interact { npc_id })
        }
        "dialogueChoice" => {
            let quest_id = extract_string(msg_data, "quest_id").unwrap_or_default();
            let choice_id = extract_string(msg_data, "choice_id").unwrap_or_default();
            Ok(ClientMessage::DialogueChoiceMsg { quest_id, choice_id })
        }
        "acceptQuest" => {
            let quest_id = extract_string(msg_data, "quest_id").unwrap_or_default();
            Ok(ClientMessage::AcceptQuest { quest_id })
        }
        "abandonQuest" => {
            let quest_id = extract_string(msg_data, "quest_id").unwrap_or_default();
            Ok(ClientMessage::AbandonQuest { quest_id })
        }
        "craft" => {
            let recipe_id = extract_string(msg_data, "recipe_id").unwrap_or_default();
            Ok(ClientMessage::Craft { recipe_id })
        }
        "equip" => {
            let slot_index = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("slot_index")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            Ok(ClientMessage::Equip { slot_index })
        }
        "unequip" => {
            let slot_type = extract_string(msg_data, "slot_type").unwrap_or_default();
            Ok(ClientMessage::Unequip { slot_type })
        }
        "dropItem" => {
            let slot_index = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("slot_index")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            let quantity = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("quantity")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u32))
                .unwrap_or(1);
            Ok(ClientMessage::DropItem { slot_index, quantity })
        }
        "dropGold" => {
            let amount = extract_i32(msg_data, "amount").unwrap_or(0);
            Ok(ClientMessage::DropGold { amount })
        }
        "swapSlots" => {
            let from_slot = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("from_slot")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            let to_slot = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("to_slot")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            Ok(ClientMessage::SwapSlots { from_slot, to_slot })
        }
        "shopBuy" => {
            let npc_id = extract_string(msg_data, "npcId").unwrap_or_default();
            let item_id = extract_string(msg_data, "itemId").unwrap_or_default();
            let quantity = extract_i32(msg_data, "quantity").unwrap_or(0);
            Ok(ClientMessage::ShopBuy { npc_id, item_id, quantity })
        }
        "shopSell" => {
            let npc_id = extract_string(msg_data, "npcId").unwrap_or_default();
            let item_id = extract_string(msg_data, "itemId").unwrap_or_default();
            let quantity = extract_i32(msg_data, "quantity").unwrap_or(0);
            Ok(ClientMessage::ShopSell { npc_id, item_id, quantity })
        }
        _ => Err(format!("Unknown message type: {}", msg_type)),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_string(value: &rmpv::Value, key: &str) -> Option<String> {
    value.as_map().and_then(|map| {
        map.iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .and_then(|(_, v)| v.as_str().map(|s| s.to_string()))
    })
}

fn extract_f32(value: &rmpv::Value, key: &str) -> Option<f32> {
    value.as_map().and_then(|map| {
        map.iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .and_then(|(_, v)| {
                v.as_f64()
                    .map(|f| f as f32)
                    .or_else(|| v.as_i64().map(|i| i as f32))
                    .or_else(|| v.as_u64().map(|u| u as f32))
            })
    })
}

fn extract_i32(value: &rmpv::Value, key: &str) -> Option<i32> {
    value.as_map().and_then(|map| {
        map.iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .and_then(|(_, v)| {
                v.as_i64()
                    .map(|i| i as i32)
                    .or_else(|| v.as_u64().map(|u| u as i32))
            })
    })
}
