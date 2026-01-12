# Shop Buy/Sell System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add shop buy/sell functionality to the merchant interaction menu with limited stock, gradual restocking, and sub-tab navigation.

**Architecture:** Extend the existing crafting menu to become a multi-purpose merchant menu. Add Shop tab with Buy/Sell sub-tabs. Shop definitions stored as TOML files with stock management. Server validates transactions, manages stock, and broadcasts updates. Client renders grid-based UI with bottom transaction bar.

**Tech Stack:** Rust (server + client), WebSocket with MessagePack protocol, TOML data files, macroquad for rendering

---

## Task 1: Create Shop Data Structures (Server)

**Files:**
- Create: `rust-server/src/shop/mod.rs`
- Create: `rust-server/src/shop/definition.rs`
- Modify: `rust-server/src/lib.rs` (add shop module)

**Step 1: Create shop module file**

Create `rust-server/src/shop/mod.rs`:

```rust
pub mod definition;
pub mod registry;

pub use definition::{ShopDefinition, ShopStockItem};
pub use registry::ShopRegistry;
```

**Step 2: Create shop definition structures**

Create `rust-server/src/shop/definition.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopDefinition {
    pub id: String,
    pub display_name: String,
    pub stock: Vec<ShopStockItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItem {
    pub item_id: String,
    pub max_quantity: i32,
    pub restock_rate: i32,
    #[serde(skip)]
    pub current_quantity: i32,
}

impl ShopDefinition {
    pub fn initialize_stock(&mut self) {
        for item in &mut self.stock {
            item.current_quantity = item.max_quantity;
        }
    }

    pub fn get_stock(&self, item_id: &str) -> Option<&ShopStockItem> {
        self.stock.iter().find(|s| s.item_id == item_id)
    }

    pub fn get_stock_mut(&mut self, item_id: &str) -> Option<&mut ShopStockItem> {
        self.stock.iter_mut().find(|s| s.item_id == item_id)
    }

    pub fn restock(&mut self) {
        for item in &mut self.stock {
            item.current_quantity = (item.current_quantity + item.restock_rate).min(item.max_quantity);
        }
    }
}
```

**Step 3: Add shop module to lib.rs**

Modify `rust-server/src/lib.rs`, add after crafting module:

```rust
pub mod shop;
```

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds with no errors

**Step 5: Commit**

```bash
git add rust-server/src/shop/ rust-server/src/lib.rs
git commit -m "feat: add shop data structures

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Create Shop Registry and TOML Loader

**Files:**
- Create: `rust-server/src/shop/registry.rs`
- Create: `rust-server/data/shops/blacksmith.toml`

**Step 1: Create shop registry**

Create `rust-server/src/shop/registry.rs`:

```rust
use super::definition::ShopDefinition;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ShopRegistry {
    shops: HashMap<String, ShopDefinition>,
}

impl ShopRegistry {
    pub fn new() -> Self {
        Self {
            shops: HashMap::new(),
        }
    }

    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut registry = Self::new();
        let dir = path.as_ref();

        if !dir.exists() {
            return Err(format!("Shop directory does not exist: {:?}", dir));
        }

        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let contents = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

                let mut shop: ShopDefinition = toml::from_str(&contents)
                    .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

                shop.initialize_stock();
                registry.shops.insert(shop.id.clone(), shop);
            }
        }

        Ok(registry)
    }

    pub fn get(&self, shop_id: &str) -> Option<&ShopDefinition> {
        self.shops.get(shop_id)
    }

    pub fn get_mut(&mut self, shop_id: &str) -> Option<&mut ShopDefinition> {
        self.shops.get_mut(shop_id)
    }
}
```

**Step 2: Create example shop definition**

Create `rust-server/data/shops/blacksmith.toml`:

```toml
id = "blacksmith"
display_name = "Blacksmith's Wares"

[[stock]]
item_id = "iron_sword"
max_quantity = 3
restock_rate = 1

[[stock]]
item_id = "leather_armor"
max_quantity = 2
restock_rate = 1

[[stock]]
item_id = "iron_helm"
max_quantity = 2
restock_rate = 1

[[stock]]
item_id = "leather_boots"
max_quantity = 4
restock_rate = 1
```

**Step 3: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add rust-server/src/shop/registry.rs rust-server/data/shops/blacksmith.toml
git commit -m "feat: add shop registry and blacksmith shop

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Add Shop Protocol Messages

**Files:**
- Modify: `rust-server/src/protocol.rs` (add ShopData, ShopOpen, ShopResult, ShopStockUpdate messages)

**Step 1: Add ShopData structures in protocol.rs**

In `rust-server/src/protocol.rs`, add after ItemDefinition structures:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopData {
    pub shop_id: String,
    pub display_name: String,
    pub buy_multiplier: f32,
    pub sell_multiplier: f32,
    pub stock: Vec<ShopStockItemData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItemData {
    pub item_id: String,
    pub quantity: i32,
    pub price: i32,
}
```

**Step 2: Add ServerMessage variants**

In `rust-server/src/protocol.rs`, in the ServerMessage enum, add after ShopOpen:

```rust
ShopData {
    npc_id: String,
    shop: ShopData,
},
ShopResult {
    success: bool,
    action: String,
    item_id: String,
    quantity: i32,
    gold_change: i32,
    error: Option<String>,
},
ShopStockUpdate {
    npc_id: String,
    item_id: String,
    new_quantity: i32,
},
```

**Step 3: Add ClientMessage variants**

In `rust-server/src/protocol.rs`, in the ClientMessage enum, add:

```rust
ShopBuy {
    npc_id: String,
    item_id: String,
    quantity: i32,
},
ShopSell {
    npc_id: String,
    item_id: String,
    quantity: i32,
},
```

**Step 4: Update encode/decode for ShopData**

In `rust-server/src/protocol.rs`, update `encode_server_message` function, add after ShopOpen case:

```rust
ServerMessage::ShopData { npc_id, shop } => {
    encode_message("shopData", &json!({
        "npcId": npc_id,
        "shop": {
            "shopId": shop.shop_id,
            "displayName": shop.display_name,
            "buyMultiplier": shop.buy_multiplier,
            "sellMultiplier": shop.sell_multiplier,
            "stock": shop.stock.iter().map(|s| json!({
                "itemId": s.item_id,
                "quantity": s.quantity,
                "price": s.price,
            })).collect::<Vec<_>>(),
        }
    }))
}
ServerMessage::ShopResult { success, action, item_id, quantity, gold_change, error } => {
    encode_message("shopResult", &json!({
        "success": success,
        "action": action,
        "itemId": item_id,
        "quantity": quantity,
        "goldChange": gold_change,
        "error": error,
    }))
}
ServerMessage::ShopStockUpdate { npc_id, item_id, new_quantity } => {
    encode_message("shopStockUpdate", &json!({
        "npcId": npc_id,
        "itemId": item_id,
        "newQuantity": new_quantity,
    }))
}
```

**Step 5: Update decode for client messages**

In `rust-server/src/protocol.rs`, in `decode_client_message` function, add cases:

```rust
"shopBuy" => {
    let npc_id = payload["npcId"].as_str().unwrap_or("").to_string();
    let item_id = payload["itemId"].as_str().unwrap_or("").to_string();
    let quantity = payload["quantity"].as_i64().unwrap_or(0) as i32;
    Some(ClientMessage::ShopBuy { npc_id, item_id, quantity })
}
"shopSell" => {
    let npc_id = payload["npcId"].as_str().unwrap_or("").to_string();
    let item_id = payload["itemId"].as_str().unwrap_or("").to_string();
    let quantity = payload["quantity"].as_i64().unwrap_or(0) as i32;
    Some(ClientMessage::ShopSell { npc_id, item_id, quantity })
}
```

**Step 6: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 7: Commit**

```bash
git add rust-server/src/protocol.rs
git commit -m "feat: add shop protocol messages

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Integrate Shop Registry into Game State

**Files:**
- Modify: `rust-server/src/game.rs` (add shop_registry field, load shops)

**Step 1: Add shop_registry to Game struct**

In `rust-server/src/game.rs`, add to Game struct after entity_registry:

```rust
shop_registry: ShopRegistry,
last_shop_restock: std::time::Instant,
```

**Step 2: Import shop modules at top of game.rs**

Add after other imports:

```rust
use crate::shop::{ShopRegistry, ShopDefinition, ShopStockItem};
```

**Step 3: Load shop registry in Game::new()**

In `rust-server/src/game.rs`, in Game::new(), add after entity_registry loading:

```rust
let shop_registry = ShopRegistry::load_from_directory("data/shops")
    .expect("Failed to load shop registry");
println!("Loaded {} shops", shop_registry.len());

let last_shop_restock = std::time::Instant::now();
```

**Step 4: Add len() method to ShopRegistry**

In `rust-server/src/shop/registry.rs`, add method:

```rust
pub fn len(&self) -> usize {
    self.shops.len()
}
```

**Step 5: Initialize shop_registry in Game struct construction**

In `rust-server/src/game.rs`, in Game::new() return statement, add fields:

```rust
shop_registry,
last_shop_restock,
```

**Step 6: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 7: Commit**

```bash
git add rust-server/src/game.rs rust-server/src/shop/registry.rs
git commit -m "feat: integrate shop registry into game state

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Implement Shop Buy Handler (Server)

**Files:**
- Modify: `rust-server/src/game.rs` (add handle_shop_buy function)

**Step 1: Add handle_shop_buy function**

In `rust-server/src/game.rs`, add after handle_craft:

```rust
fn handle_shop_buy(&mut self, player_id: &str, npc_id: &str, item_id: &str, quantity: i32) {
    // Validate player
    let player = match self.players.get_mut(player_id) {
        Some(p) if p.stats.hp > 0 => p,
        _ => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Player not found or dead".to_string()));
            return;
        }
    };

    // Validate NPC
    let npc = match self.npcs.get(npc_id) {
        Some(n) if n.stats.hp > 0 => n,
        _ => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Merchant not found or dead".to_string()));
            return;
        }
    };

    // Check distance
    let distance = ((player.x - npc.x).powi(2) + (player.y - npc.y).powi(2)).sqrt();
    if distance > 2.5 {
        self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Too far from merchant".to_string()));
        return;
    }

    // Get merchant config
    let merchant_config = match self.entity_registry.get(&npc.entity_type)
        .and_then(|proto| proto.merchant.as_ref()) {
        Some(config) => config,
        None => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("NPC is not a merchant".to_string()));
            return;
        }
    };

    // Get shop
    let shop = match self.shop_registry.get_mut(&merchant_config.shop_id) {
        Some(s) => s,
        None => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Shop not found".to_string()));
            return;
        }
    };

    // Check stock
    let stock_item = match shop.get_stock_mut(item_id) {
        Some(s) if s.current_quantity >= quantity => s,
        Some(_) => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Insufficient stock".to_string()));
            return;
        }
        None => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Item not available".to_string()));
            return;
        }
    };

    // Get item definition
    let item_def = match self.item_registry.get(item_id) {
        Some(def) => def,
        None => {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Item not found".to_string()));
            return;
        }
    };

    // Calculate price
    let total_price = (item_def.base_price as f32 * merchant_config.sell_multiplier * quantity as f32) as i32;

    // Check gold
    let player = self.players.get_mut(player_id).unwrap();
    if player.inventory.gold < total_price {
        self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some(format!("Need {} gold", total_price)));
        return;
    }

    // Check inventory space
    if !player.inventory.has_space_for(item_id, quantity, &self.item_registry) {
        self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Not enough inventory space".to_string()));
        return;
    }

    // Execute transaction
    player.inventory.gold -= total_price;
    player.inventory.add_item(item_id, quantity, &self.item_registry);

    // Update stock
    let shop = self.shop_registry.get_mut(&merchant_config.shop_id).unwrap();
    let stock_item = shop.get_stock_mut(item_id).unwrap();
    stock_item.current_quantity -= quantity;
    let new_quantity = stock_item.current_quantity;

    // Send results
    self.send_shop_result(player_id, true, "buy", item_id, quantity, -total_price, None);
    self.send_inventory_update(player_id);
    self.broadcast_shop_stock_update(npc_id, item_id, new_quantity);
}

fn send_shop_result(&mut self, player_id: &str, success: bool, action: &str, item_id: &str, quantity: i32, gold_change: i32, error: Option<String>) {
    if let Some(client_id) = self.players.get(player_id).map(|p| p.client_id) {
        self.server.send_message(
            client_id,
            ServerMessage::ShopResult {
                success,
                action: action.to_string(),
                item_id: item_id.to_string(),
                quantity,
                gold_change,
                error,
            },
        );
    }
}

fn broadcast_shop_stock_update(&mut self, npc_id: &str, item_id: &str, new_quantity: i32) {
    self.server.broadcast(ServerMessage::ShopStockUpdate {
        npc_id: npc_id.to_string(),
        item_id: item_id.to_string(),
        new_quantity,
    });
}
```

**Step 2: Add ClientMessage::ShopBuy handler in message processing**

In `rust-server/src/game.rs`, in the message processing section (handle_client_message or similar), add case:

```rust
ClientMessage::ShopBuy { npc_id, item_id, quantity } => {
    self.handle_shop_buy(&player_id, &npc_id, &item_id, quantity);
}
```

**Step 3: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add rust-server/src/game.rs
git commit -m "feat: implement shop buy handler

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Implement Shop Sell Handler (Server)

**Files:**
- Modify: `rust-server/src/game.rs` (add handle_shop_sell function)

**Step 1: Add handle_shop_sell function**

In `rust-server/src/game.rs`, add after handle_shop_buy:

```rust
fn handle_shop_sell(&mut self, player_id: &str, npc_id: &str, item_id: &str, quantity: i32) {
    // Validate player
    let player = match self.players.get_mut(player_id) {
        Some(p) if p.stats.hp > 0 => p,
        _ => {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Player not found or dead".to_string()));
            return;
        }
    };

    // Validate NPC
    let npc = match self.npcs.get(npc_id) {
        Some(n) if n.stats.hp > 0 => n,
        _ => {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Merchant not found or dead".to_string()));
            return;
        }
    };

    // Check distance
    let distance = ((player.x - npc.x).powi(2) + (player.y - npc.y).powi(2)).sqrt();
    if distance > 2.5 {
        self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Too far from merchant".to_string()));
        return;
    }

    // Get merchant config
    let merchant_config = match self.entity_registry.get(&npc.entity_type)
        .and_then(|proto| proto.merchant.as_ref()) {
        Some(config) => config,
        None => {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("NPC is not a merchant".to_string()));
            return;
        }
    };

    // Get item definition
    let item_def = match self.item_registry.get(item_id) {
        Some(def) => def,
        None => {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Item not found".to_string()));
            return;
        }
    };

    // Check if item is sellable
    if !item_def.sellable {
        self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("This item cannot be sold".to_string()));
        return;
    }

    // Check player has item
    let player = self.players.get_mut(player_id).unwrap();
    if !player.inventory.has_item(item_id, quantity) {
        self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Not enough items".to_string()));
        return;
    }

    // Calculate price
    let total_price = (item_def.base_price as f32 * merchant_config.buy_multiplier * quantity as f32) as i32;

    // Execute transaction
    player.inventory.remove_item(item_id, quantity);
    player.inventory.gold += total_price;

    // Send results
    self.send_shop_result(player_id, true, "sell", item_id, quantity, total_price, None);
    self.send_inventory_update(player_id);
}
```

**Step 2: Add ClientMessage::ShopSell handler**

In `rust-server/src/game.rs`, in the message processing section, add case:

```rust
ClientMessage::ShopSell { npc_id, item_id, quantity } => {
    self.handle_shop_sell(&player_id, &npc_id, &item_id, quantity);
}
```

**Step 3: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add rust-server/src/game.rs
git commit -m "feat: implement shop sell handler

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Update ShopOpen Handler to Send Shop Data

**Files:**
- Modify: `rust-server/src/game.rs` (update handle_npc_interact to send ShopData)

**Step 1: Update NPC interact handler**

In `rust-server/src/game.rs`, find the handle_npc_interact function (around line 1620-1690), replace ShopOpen message sending with:

```rust
// OLD:
self.server.send_message(client_id, ServerMessage::ShopOpen { npc_id: npc_id.to_string() });

// NEW:
if let Some(merchant_config) = prototype.merchant.as_ref() {
    if let Some(shop) = self.shop_registry.get(&merchant_config.shop_id) {
        let shop_data = protocol::ShopData {
            shop_id: shop.id.clone(),
            display_name: shop.display_name.clone(),
            buy_multiplier: merchant_config.buy_multiplier,
            sell_multiplier: merchant_config.sell_multiplier,
            stock: shop.stock.iter().map(|s| {
                let item_def = self.item_registry.get(&s.item_id);
                let base_price = item_def.map(|d| d.base_price).unwrap_or(0);
                let price = (base_price as f32 * merchant_config.sell_multiplier) as i32;
                protocol::ShopStockItemData {
                    item_id: s.item_id.clone(),
                    quantity: s.current_quantity,
                    price,
                }
            }).collect(),
        };
        self.server.send_message(
            client_id,
            ServerMessage::ShopData {
                npc_id: npc_id.to_string(),
                shop: shop_data,
            },
        );
    }
}
```

**Step 2: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add rust-server/src/game.rs
git commit -m "feat: send shop data when opening shop

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Implement Shop Restock System

**Files:**
- Modify: `rust-server/src/game.rs` (add restock tick in game loop)

**Step 1: Add restock logic to game loop**

In `rust-server/src/game.rs`, in the Game::update() or tick() function, add after other periodic checks:

```rust
// Shop restock tick
if self.last_shop_restock.elapsed().as_secs() >= 60 {
    self.restock_shops();
    self.last_shop_restock = std::time::Instant::now();
}
```

**Step 2: Implement restock_shops function**

In `rust-server/src/game.rs`, add function:

```rust
fn restock_shops(&mut self) {
    // Get merchant configs with restock intervals
    let mut shops_to_restock = Vec::new();

    for (entity_type, proto) in self.entity_registry.iter() {
        if let Some(merchant_config) = &proto.merchant {
            if let Some(interval_minutes) = merchant_config.restock_interval_minutes {
                // Check if enough time has passed (simplified: restock every interval)
                shops_to_restock.push((merchant_config.shop_id.clone(), interval_minutes));
            }
        }
    }

    // Restock shops
    for (shop_id, _interval) in shops_to_restock {
        if let Some(shop) = self.shop_registry.get_mut(&shop_id) {
            shop.restock();

            // Broadcast stock updates for each item
            for stock_item in &shop.stock {
                // Find NPCs with this shop
                for (npc_id, npc) in &self.npcs {
                    if let Some(proto) = self.entity_registry.get(&npc.entity_type) {
                        if let Some(merchant_config) = &proto.merchant {
                            if merchant_config.shop_id == shop_id {
                                self.broadcast_shop_stock_update(
                                    npc_id,
                                    &stock_item.item_id,
                                    stock_item.current_quantity,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
```

**Step 3: Add iter() method to EntityRegistry**

In `rust-server/src/entity/registry.rs`, add if not present:

```rust
pub fn iter(&self) -> impl Iterator<Item = (&String, &EntityPrototype)> {
    self.prototypes.iter()
}
```

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add rust-server/src/game.rs rust-server/src/entity/registry.rs
git commit -m "feat: implement shop restock system

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Add Client-Side Shop Data Structures

**Files:**
- Modify: `client/src/game/state.rs` (add shop UI state fields)
- Create: `client/src/game/shop.rs` (shop data structures)
- Modify: `client/src/game/mod.rs` (add shop module)

**Step 1: Create shop data structures**

Create `client/src/game/shop.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopData {
    pub shop_id: String,
    pub display_name: String,
    pub buy_multiplier: f32,
    pub sell_multiplier: f32,
    pub stock: Vec<ShopStockItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItem {
    pub item_id: String,
    pub quantity: i32,
    pub price: i32,
}

impl ShopData {
    pub fn get_stock(&self, item_id: &str) -> Option<&ShopStockItem> {
        self.stock.iter().find(|s| s.item_id == item_id)
    }

    pub fn update_stock(&mut self, item_id: &str, new_quantity: i32) {
        if let Some(item) = self.stock.iter_mut().find(|s| s.item_id == item_id) {
            item.quantity = new_quantity;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopSubTab {
    Buy,
    Sell,
}
```

**Step 2: Add shop module to game mod**

In `client/src/game/mod.rs`, add:

```rust
pub mod shop;
pub use shop::{ShopData, ShopStockItem, ShopSubTab};
```

**Step 3: Add shop state fields to GameState**

In `client/src/game/state.rs`, in the UIState or GameState struct, add fields:

```rust
// Shop UI state
pub shop_tab_active: bool,  // true = Shop, false = Recipes
pub shop_sub_tab: ShopSubTab,
pub shop_selected_item: Option<usize>,
pub shop_quantity: i32,
pub shop_data: Option<ShopData>,
pub shop_npc_id: Option<String>,
```

**Step 4: Import ShopSubTab**

At top of `client/src/game/state.rs`:

```rust
use super::shop::ShopSubTab;
```

**Step 5: Initialize shop state in GameState::new()**

In `client/src/game/state.rs`, in initialization:

```rust
shop_tab_active: false,
shop_sub_tab: ShopSubTab::Buy,
shop_selected_item: None,
shop_quantity: 1,
shop_data: None,
shop_npc_id: None,
```

**Step 6: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 7: Commit**

```bash
git add client/src/game/shop.rs client/src/game/mod.rs client/src/game/state.rs
git commit -m "feat: add client shop data structures

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 10: Add Client Network Message Handlers

**Files:**
- Modify: `client/src/network/messages.rs` (add ShopBuy, ShopSell messages)
- Modify: `client/src/network/protocol.rs` (add encoding)

**Step 1: Add ClientMessage variants**

In `client/src/network/messages.rs`, add to ClientMessage enum:

```rust
ShopBuy {
    npc_id: String,
    item_id: String,
    quantity: i32,
},
ShopSell {
    npc_id: String,
    item_id: String,
    quantity: i32,
},
```

**Step 2: Add encoding for shop messages**

In `client/src/network/protocol.rs`, in to_protocol() function for ClientMessage, add cases:

```rust
ClientMessage::ShopBuy { npc_id, item_id, quantity } => {
    ("shopBuy", json!({
        "npcId": npc_id,
        "itemId": item_id,
        "quantity": quantity,
    }))
}
ClientMessage::ShopSell { npc_id, item_id, quantity } => {
    ("shopSell", json!({
        "npcId": npc_id,
        "itemId": item_id,
        "quantity": quantity,
    }))
}
```

**Step 3: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add client/src/network/messages.rs client/src/network/protocol.rs
git commit -m "feat: add client shop messages

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 11: Handle Server Shop Messages in Client

**Files:**
- Modify: `client/src/network/client.rs` (add shopData, shopResult, shopStockUpdate handlers)

**Step 1: Add shopData handler**

In `client/src/network/client.rs`, in the message handling section (around line 1102-1179), replace "ShopOpen" handler with:

```rust
"shopData" => {
    if let Some(npc_id) = data["npcId"].as_str() {
        if let Some(shop_obj) = data["shop"].as_object() {
            let stock: Vec<ShopStockItem> = shop_obj["stock"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|s| {
                    Some(ShopStockItem {
                        item_id: s["itemId"].as_str()?.to_string(),
                        quantity: s["quantity"].as_i64()? as i32,
                        price: s["price"].as_i64()? as i32,
                    })
                })
                .collect();

            let shop_data = ShopData {
                shop_id: shop_obj["shopId"].as_str().unwrap_or("").to_string(),
                display_name: shop_obj["displayName"].as_str().unwrap_or("Shop").to_string(),
                buy_multiplier: shop_obj["buyMultiplier"].as_f64().unwrap_or(0.5) as f32,
                sell_multiplier: shop_obj["sellMultiplier"].as_f64().unwrap_or(1.0) as f32,
                stock,
            };

            game_state.ui_state.crafting_open = true;
            game_state.ui_state.shop_tab_active = true;
            game_state.ui_state.shop_sub_tab = ShopSubTab::Buy;
            game_state.ui_state.shop_data = Some(shop_data);
            game_state.ui_state.shop_npc_id = Some(npc_id.to_string());
            game_state.ui_state.shop_selected_item = None;
            game_state.ui_state.shop_quantity = 1;
        }
    }
}
```

**Step 2: Add shopResult handler**

In `client/src/network/client.rs`, add new message handler:

```rust
"shopResult" => {
    let success = data["success"].as_bool().unwrap_or(false);
    let action = data["action"].as_str().unwrap_or("");

    if success {
        let item_id = data["itemId"].as_str().unwrap_or("");
        let quantity = data["quantity"].as_i64().unwrap_or(0);
        let gold_change = data["goldChange"].as_i64().unwrap_or(0);

        let action_text = if action == "buy" { "Bought" } else { "Sold" };
        let item_name = game_state.item_registry.get_display_name(item_id);
        game_state.add_notification(&format!("{} {} {} for {} gold", action_text, quantity, item_name, gold_change.abs()));

        // Reset selection
        game_state.ui_state.shop_selected_item = None;
        game_state.ui_state.shop_quantity = 1;
    } else if let Some(error) = data["error"].as_str() {
        game_state.add_chat_message("System", error);
    }
}
```

**Step 3: Add shopStockUpdate handler**

In `client/src/network/client.rs`, add new message handler:

```rust
"shopStockUpdate" => {
    if let Some(npc_id) = data["npcId"].as_str() {
        if let Some(item_id) = data["itemId"].as_str() {
            let new_quantity = data["newQuantity"].as_i64().unwrap_or(0) as i32;

            // Update stock if shop is open for this NPC
            if game_state.ui_state.shop_npc_id.as_deref() == Some(npc_id) {
                if let Some(shop_data) = &mut game_state.ui_state.shop_data {
                    shop_data.update_stock(item_id, new_quantity);
                }
            }
        }
    }
}
```

**Step 4: Add imports at top of client.rs**

Add:

```rust
use crate::game::{ShopData, ShopStockItem, ShopSubTab};
```

**Step 5: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 6: Commit**

```bash
git add client/src/network/client.rs
git commit -m "feat: handle shop server messages in client

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 12: Create Shop UI Rendering Module

**Files:**
- Create: `client/src/render/ui/shop.rs`
- Modify: `client/src/render/ui/mod.rs` (add shop module)

**Step 1: Create shop UI module**

Create `client/src/render/ui/shop.rs`:

```rust
use macroquad::prelude::*;
use crate::game::{GameState, ShopSubTab};
use crate::ui::layout::{UiLayout, UiElementId};

const PANEL_WIDTH: f32 = 600.0;
const PANEL_HEIGHT: f32 = 500.0;
const ITEM_GRID_COLS: usize = 5;
const ITEM_SIZE: f32 = 80.0;
const ITEM_PADDING: f32 = 10.0;
const BOTTOM_BAR_HEIGHT: f32 = 80.0;

pub fn render_shop_tab(
    game_state: &GameState,
    ui_layout: &mut UiLayout,
    panel_x: f32,
    panel_y: f32,
) {
    let shop_data = match &game_state.ui_state.shop_data {
        Some(data) => data,
        None => return,
    };

    // Render sub-tabs
    render_sub_tabs(game_state, ui_layout, panel_x, panel_y + 60.0);

    // Render content based on active sub-tab
    let content_y = panel_y + 100.0;
    match game_state.ui_state.shop_sub_tab {
        ShopSubTab::Buy => render_buy_view(game_state, shop_data, ui_layout, panel_x, content_y),
        ShopSubTab::Sell => render_sell_view(game_state, shop_data, ui_layout, panel_x, content_y),
    }

    // Render bottom transaction bar
    render_transaction_bar(game_state, shop_data, ui_layout, panel_x, panel_y);
}

fn render_sub_tabs(
    game_state: &GameState,
    ui_layout: &mut UiLayout,
    x: f32,
    y: f32,
) {
    let tab_width = 100.0;
    let tab_height = 30.0;

    // Buy tab
    let buy_active = game_state.ui_state.shop_sub_tab == ShopSubTab::Buy;
    let buy_color = if buy_active { GOLD } else { DARKGRAY };
    draw_rectangle(x + 10.0, y, tab_width, tab_height, buy_color);
    draw_text("BUY", x + 35.0, y + 20.0, 20.0, BLACK);
    ui_layout.add_element(UiElementId::ShopSubTab(0), x + 10.0, y, tab_width, tab_height);

    // Sell tab
    let sell_active = game_state.ui_state.shop_sub_tab == ShopSubTab::Sell;
    let sell_color = if sell_active { GOLD } else { DARKGRAY };
    draw_rectangle(x + 120.0, y, tab_width, tab_height, sell_color);
    draw_text("SELL", x + 140.0, y + 20.0, 20.0, BLACK);
    ui_layout.add_element(UiElementId::ShopSubTab(1), x + 120.0, y, tab_width, tab_height);
}

fn render_buy_view(
    game_state: &GameState,
    shop_data: &crate::game::ShopData,
    ui_layout: &mut UiLayout,
    x: f32,
    y: f32,
) {
    let grid_start_x = x + 20.0;
    let grid_start_y = y;

    for (idx, stock_item) in shop_data.stock.iter().enumerate() {
        let col = idx % ITEM_GRID_COLS;
        let row = idx / ITEM_GRID_COLS;
        let item_x = grid_start_x + (col as f32) * (ITEM_SIZE + ITEM_PADDING);
        let item_y = grid_start_y + (row as f32) * (ITEM_SIZE + ITEM_PADDING);

        // Item background
        let is_selected = game_state.ui_state.shop_selected_item == Some(idx);
        let bg_color = if stock_item.quantity == 0 {
            DARKGRAY
        } else if is_selected {
            YELLOW
        } else {
            LIGHTGRAY
        };
        draw_rectangle(item_x, item_y, ITEM_SIZE, ITEM_SIZE, bg_color);

        // Item sprite (placeholder)
        let sprite_color = if stock_item.quantity == 0 { GRAY } else { WHITE };
        draw_rectangle(item_x + 10.0, item_y + 10.0, 60.0, 40.0, sprite_color);

        // Stock count
        let count_text = format!("x{}", stock_item.quantity);
        draw_text(&count_text, item_x + 5.0, item_y + 65.0, 16.0, BLACK);

        // Price
        let price_text = format!("{}g", stock_item.price);
        draw_text(&price_text, item_x + 5.0, item_y + 78.0, 14.0, DARKGREEN);

        // Register click area
        ui_layout.add_element(
            UiElementId::ShopBuyItem(idx),
            item_x,
            item_y,
            ITEM_SIZE,
            ITEM_SIZE,
        );
    }
}

fn render_sell_view(
    game_state: &GameState,
    shop_data: &crate::game::ShopData,
    ui_layout: &mut UiLayout,
    x: f32,
    y: f32,
) {
    let grid_start_x = x + 20.0;
    let grid_start_y = y;

    let mut item_idx = 0;
    for (slot_idx, slot) in game_state.player_inventory.slots.iter().enumerate() {
        if let Some(inv_slot) = slot {
            // Check if item is sellable
            if let Some(item_def) = game_state.item_registry.get(&inv_slot.item_id) {
                if !item_def.sellable {
                    continue;
                }

                let col = item_idx % ITEM_GRID_COLS;
                let row = item_idx / ITEM_GRID_COLS;
                let item_x = grid_start_x + (col as f32) * (ITEM_SIZE + ITEM_PADDING);
                let item_y = grid_start_y + (row as f32) * (ITEM_SIZE + ITEM_PADDING);

                // Item background
                let is_selected = game_state.ui_state.shop_selected_item == Some(slot_idx);
                let bg_color = if is_selected { YELLOW } else { LIGHTGRAY };
                draw_rectangle(item_x, item_y, ITEM_SIZE, ITEM_SIZE, bg_color);

                // Item sprite (placeholder)
                draw_rectangle(item_x + 10.0, item_y + 10.0, 60.0, 40.0, WHITE);

                // Quantity
                let count_text = format!("x{}", inv_slot.quantity);
                draw_text(&count_text, item_x + 5.0, item_y + 65.0, 16.0, BLACK);

                // Sell price
                let sell_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                let price_text = format!("{}g", sell_price);
                draw_text(&price_text, item_x + 5.0, item_y + 78.0, 14.0, DARKGREEN);

                // Register click area (use slot_idx to map back to inventory)
                ui_layout.add_element(
                    UiElementId::ShopSellItem(slot_idx),
                    item_x,
                    item_y,
                    ITEM_SIZE,
                    ITEM_SIZE,
                );

                item_idx += 1;
            }
        }
    }

    // Empty state
    if item_idx == 0 {
        draw_text("No sellable items", x + 200.0, y + 100.0, 20.0, DARKGRAY);
    }
}

fn render_transaction_bar(
    game_state: &GameState,
    shop_data: &crate::game::ShopData,
    ui_layout: &mut UiLayout,
    panel_x: f32,
    panel_y: f32,
) {
    let bar_y = panel_y + PANEL_HEIGHT - BOTTOM_BAR_HEIGHT;

    // Background
    draw_rectangle(panel_x, bar_y, PANEL_WIDTH, BOTTOM_BAR_HEIGHT, DARKGRAY);

    // Gold display
    let gold_text = format!("Gold: {}", game_state.player_inventory.gold);
    draw_text(&gold_text, panel_x + 10.0, bar_y + 70.0, 18.0, GOLD);

    // If item selected, show transaction controls
    if let Some(selected_idx) = game_state.ui_state.shop_selected_item {
        let (item_name, total_price, max_quantity) = match game_state.ui_state.shop_sub_tab {
            ShopSubTab::Buy => {
                if let Some(stock_item) = shop_data.stock.get(selected_idx) {
                    let name = game_state.item_registry.get_display_name(&stock_item.item_id);
                    let total = stock_item.price * game_state.ui_state.shop_quantity;
                    (name, total, stock_item.quantity)
                } else {
                    return;
                }
            }
            ShopSubTab::Sell => {
                if let Some(Some(inv_slot)) = game_state.player_inventory.slots.get(selected_idx) {
                    let name = game_state.item_registry.get_display_name(&inv_slot.item_id);
                    let item_def = game_state.item_registry.get(&inv_slot.item_id).unwrap();
                    let unit_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                    let total = unit_price * game_state.ui_state.shop_quantity;
                    (name, total, inv_slot.quantity)
                } else {
                    return;
                }
            }
        };

        // Item name
        draw_text(&item_name, panel_x + 20.0, bar_y + 25.0, 18.0, WHITE);

        // Quantity controls
        let qty_x = panel_x + 250.0;

        // Minus button
        draw_rectangle(qty_x, bar_y + 15.0, 30.0, 30.0, RED);
        draw_text("-", qty_x + 10.0, bar_y + 35.0, 24.0, WHITE);
        ui_layout.add_element(UiElementId::ShopQuantityMinus, qty_x, bar_y + 15.0, 30.0, 30.0);

        // Quantity display
        let qty_text = format!("{}", game_state.ui_state.shop_quantity);
        draw_text(&qty_text, qty_x + 45.0, bar_y + 35.0, 20.0, WHITE);

        // Plus button
        draw_rectangle(qty_x + 80.0, bar_y + 15.0, 30.0, 30.0, GREEN);
        draw_text("+", qty_x + 90.0, bar_y + 35.0, 24.0, WHITE);
        ui_layout.add_element(UiElementId::ShopQuantityPlus, qty_x + 80.0, bar_y + 15.0, 30.0, 30.0);

        // Total price
        let price_text = format!("{}g", total_price);
        draw_text(&price_text, panel_x + 400.0, bar_y + 35.0, 20.0, GOLD);

        // Confirm button
        let button_text = if game_state.ui_state.shop_sub_tab == ShopSubTab::Buy {
            "BUY"
        } else {
            "SELL"
        };
        let button_x = panel_x + 480.0;
        let can_afford = game_state.ui_state.shop_sub_tab == ShopSubTab::Sell ||
                         game_state.player_inventory.gold >= total_price;
        let valid_qty = game_state.ui_state.shop_quantity > 0 &&
                        game_state.ui_state.shop_quantity <= max_quantity;
        let button_color = if can_afford && valid_qty { GREEN } else { GRAY };

        draw_rectangle(button_x, bar_y + 15.0, 100.0, 40.0, button_color);
        draw_text(button_text, button_x + 25.0, bar_y + 42.0, 20.0, WHITE);
        ui_layout.add_element(UiElementId::ShopConfirmButton, button_x, bar_y + 15.0, 100.0, 40.0);
    }
}
```

**Step 2: Add shop module to ui mod**

In `client/src/render/ui/mod.rs`, add:

```rust
pub mod shop;
```

**Step 3: Verify compilation**

Run: `cd client && cargo check`
Expected: Will likely fail due to missing UiElementId variants - that's OK, we'll add them next

**Step 4: Commit**

```bash
git add client/src/render/ui/shop.rs client/src/render/ui/mod.rs
git commit -m "feat: create shop UI rendering module

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 13: Add UI Element IDs for Shop

**Files:**
- Modify: `client/src/ui/layout.rs` (add ShopSubTab, ShopBuyItem, ShopSellItem, etc.)

**Step 1: Add UiElementId variants**

In `client/src/ui/layout.rs`, in the UiElementId enum, add:

```rust
ShopSubTab(usize),
ShopBuyItem(usize),
ShopSellItem(usize),
ShopQuantityMinus,
ShopQuantityPlus,
ShopConfirmButton,
```

**Step 2: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add client/src/ui/layout.rs
git commit -m "feat: add shop UI element IDs

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 14: Integrate Shop Tab into Crafting Menu

**Files:**
- Modify: `client/src/render/ui/crafting.rs` (add shop/recipes tab switching, render shop when active)

**Step 1: Update crafting UI to show main tabs**

In `client/src/render/ui/crafting.rs`, modify the render function to add main tab headers:

```rust
// Add after header rendering, before content
pub fn render(game_state: &GameState, ui_layout: &mut UiLayout) {
    // ... existing panel setup ...

    // Main tabs (Shop / Recipes)
    let tab_y = panel_y + 40.0;
    let tab_width = 100.0;
    let tab_height = 30.0;

    // Shop tab
    let shop_active = game_state.ui_state.shop_tab_active;
    let shop_color = if shop_active { GOLD } else { DARKGRAY };
    draw_rectangle(panel_x + 10.0, tab_y, tab_width, tab_height, shop_color);
    draw_text("SHOP", panel_x + 30.0, tab_y + 20.0, 20.0, BLACK);
    ui_layout.add_element(UiElementId::MainTab(0), panel_x + 10.0, tab_y, tab_width, tab_height);

    // Recipes tab
    let recipes_active = !game_state.ui_state.shop_tab_active;
    let recipes_color = if recipes_active { GOLD } else { DARKGRAY };
    draw_rectangle(panel_x + 120.0, tab_y, tab_width, tab_height, recipes_color);
    draw_text("RECIPES", panel_x + 125.0, tab_y + 20.0, 20.0, BLACK);
    ui_layout.add_element(UiElementId::MainTab(1), panel_x + 120.0, tab_y, tab_width, tab_height);

    // Render content based on active tab
    if shop_active && game_state.ui_state.shop_data.is_some() {
        super::shop::render_shop_tab(game_state, ui_layout, panel_x, panel_y);
    } else {
        // Existing recipe rendering code
        render_recipes_tab(game_state, ui_layout, panel_x, panel_y);
    }
}
```

**Step 2: Extract existing recipe rendering into function**

In `client/src/render/ui/crafting.rs`, wrap existing recipe rendering in a function:

```rust
fn render_recipes_tab(game_state: &GameState, ui_layout: &mut UiLayout, panel_x: f32, panel_y: f32) {
    // Move all existing recipe rendering code here
    // (categories, recipe list, craft button, etc.)
}
```

**Step 3: Update header to show merchant name**

In `client/src/render/ui/crafting.rs`, update header text:

```rust
let header_text = if let Some(npc_id) = &game_state.ui_state.shop_npc_id {
    // Get NPC name from entity registry or use "MERCHANT"
    "MERCHANT".to_string()
} else {
    "CRAFTING".to_string()
};
draw_text(&header_text, panel_x + 20.0, panel_y + 25.0, 24.0, WHITE);
```

**Step 4: Add MainTab to UiElementId**

In `client/src/ui/layout.rs`, add:

```rust
MainTab(usize),
```

**Step 5: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 6: Commit**

```bash
git add client/src/render/ui/crafting.rs client/src/ui/layout.rs
git commit -m "feat: integrate shop tab into crafting menu

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 15: Add Input Handling for Shop UI

**Files:**
- Modify: `client/src/input/handler.rs` (add shop tab switching, item selection, quantity adjust, confirm)

**Step 1: Add main tab switching**

In `client/src/input/handler.rs`, in mouse click handling for crafting UI, add:

```rust
UiElementId::MainTab(idx) => {
    game_state.ui_state.shop_tab_active = idx == 0;
    if idx == 0 {
        // Switched to Shop
        game_state.ui_state.shop_selected_item = None;
        game_state.ui_state.shop_quantity = 1;
    } else {
        // Switched to Recipes
        game_state.ui_state.crafting_selected_recipe = 0;
    }
}
```

**Step 2: Add shop sub-tab switching**

Add:

```rust
UiElementId::ShopSubTab(idx) => {
    game_state.ui_state.shop_sub_tab = if idx == 0 {
        ShopSubTab::Buy
    } else {
        ShopSubTab::Sell
    };
    game_state.ui_state.shop_selected_item = None;
    game_state.ui_state.shop_quantity = 1;
}
```

**Step 3: Add buy item selection**

Add:

```rust
UiElementId::ShopBuyItem(idx) => {
    if let Some(shop_data) = &game_state.ui_state.shop_data {
        if shop_data.stock.get(idx).map(|s| s.quantity > 0).unwrap_or(false) {
            game_state.ui_state.shop_selected_item = Some(idx);
            game_state.ui_state.shop_quantity = 1;
        }
    }
}
```

**Step 4: Add sell item selection**

Add:

```rust
UiElementId::ShopSellItem(slot_idx) => {
    game_state.ui_state.shop_selected_item = Some(slot_idx);
    game_state.ui_state.shop_quantity = 1;
}
```

**Step 5: Add quantity controls**

Add:

```rust
UiElementId::ShopQuantityMinus => {
    if game_state.ui_state.shop_quantity > 1 {
        game_state.ui_state.shop_quantity -= 1;
    }
}
UiElementId::ShopQuantityPlus => {
    let max_quantity = match game_state.ui_state.shop_sub_tab {
        ShopSubTab::Buy => {
            game_state.ui_state.shop_selected_item
                .and_then(|idx| game_state.ui_state.shop_data.as_ref()
                    .and_then(|shop| shop.stock.get(idx).map(|s| s.quantity)))
                .unwrap_or(0)
        }
        ShopSubTab::Sell => {
            game_state.ui_state.shop_selected_item
                .and_then(|idx| game_state.player_inventory.slots.get(idx)
                    .and_then(|s| s.as_ref().map(|s| s.quantity)))
                .unwrap_or(0)
        }
    };
    if game_state.ui_state.shop_quantity < max_quantity {
        game_state.ui_state.shop_quantity += 1;
    }
}
```

**Step 6: Add confirm button handler**

Add:

```rust
UiElementId::ShopConfirmButton => {
    if let (Some(npc_id), Some(selected_idx)) = (
        &game_state.ui_state.shop_npc_id,
        game_state.ui_state.shop_selected_item,
    ) {
        let quantity = game_state.ui_state.shop_quantity;
        match game_state.ui_state.shop_sub_tab {
            ShopSubTab::Buy => {
                if let Some(shop_data) = &game_state.ui_state.shop_data {
                    if let Some(stock_item) = shop_data.stock.get(selected_idx) {
                        client.send(&ClientMessage::ShopBuy {
                            npc_id: npc_id.clone(),
                            item_id: stock_item.item_id.clone(),
                            quantity,
                        });
                    }
                }
            }
            ShopSubTab::Sell => {
                if let Some(Some(inv_slot)) = game_state.player_inventory.slots.get(selected_idx) {
                    client.send(&ClientMessage::ShopSell {
                        npc_id: npc_id.clone(),
                        item_id: inv_slot.item_id.clone(),
                        quantity,
                    });
                }
            }
        }
    }
}
```

**Step 7: Add keyboard controls**

In keyboard handling for crafting UI, add:

```rust
// Tab key to switch shop sub-tabs
if is_key_pressed(KeyCode::Tab) && game_state.ui_state.shop_tab_active {
    game_state.ui_state.shop_sub_tab = match game_state.ui_state.shop_sub_tab {
        ShopSubTab::Buy => ShopSubTab::Sell,
        ShopSubTab::Sell => ShopSubTab::Buy,
    };
    game_state.ui_state.shop_selected_item = None;
    game_state.ui_state.shop_quantity = 1;
}

// Q/E to switch main tabs
if is_key_pressed(KeyCode::Q) {
    game_state.ui_state.shop_tab_active = true;
}
if is_key_pressed(KeyCode::E) && game_state.ui_state.shop_data.is_some() {
    game_state.ui_state.shop_tab_active = false;
}
```

**Step 8: Add imports**

At top of `client/src/input/handler.rs`:

```rust
use crate::game::ShopSubTab;
use crate::network::messages::ClientMessage;
```

**Step 9: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 10: Commit**

```bash
git add client/src/input/handler.rs
git commit -m "feat: add shop UI input handling

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 16: Update Merchant Entity Config

**Files:**
- Modify: `rust-server/data/entities/npcs/villagers.toml` (add merchant config to blacksmith)

**Step 1: Add merchant config to blacksmith**

In `rust-server/data/entities/npcs/villagers.toml`, update blacksmith section:

```toml
[blacksmith.merchant]
shop_id = "blacksmith"
buy_multiplier = 0.5
sell_multiplier = 1.0
restock_interval_minutes = 5
```

**Step 2: Verify file syntax**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds, TOML loads correctly

**Step 3: Commit**

```bash
git add rust-server/data/entities/npcs/villagers.toml
git commit -m "feat: configure blacksmith as merchant

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 17: Test Shop Buying Flow

**Files:**
- None (manual testing)

**Step 1: Start the server**

Run: `cd rust-server && cargo run`
Expected: Server starts, loads 1 shop

**Step 2: Start the client**

Run: `cd client && cargo run`
Expected: Client connects successfully

**Step 3: Approach blacksmith NPC and interact**

- Walk to blacksmith
- Press interact key (likely F or E)
Expected: Merchant menu opens with Shop tab active, showing Buy sub-tab

**Step 4: Verify shop stock displays**

Expected: See 4 items (iron_sword, leather_armor, iron_helm, leather_boots) with quantities and prices

**Step 5: Select an item and buy**

- Click an item
- Adjust quantity with +/-
- Click BUY button
Expected: Gold deducted, item added to inventory, stock decreased

**Step 6: Verify insufficient gold handling**

- Try to buy expensive item without enough gold
Expected: Transaction fails with error message

**Step 7: Verify out-of-stock handling**

- Buy all stock of one item
Expected: Item shows x0, greyed out, cannot be selected

**Step 8: Document results**

Create test notes in: `docs/testing/2026-01-12-shop-buy-test.md`

---

## Task 18: Test Shop Selling Flow

**Files:**
- None (manual testing)

**Step 1: Switch to Sell tab**

- With shop open, click SELL sub-tab or press Tab
Expected: View switches to show player's sellable inventory items

**Step 2: Verify only sellable items shown**

Expected: Quest items and non-sellable items (like gold) are hidden

**Step 3: Select and sell an item**

- Click a sellable item
- Adjust quantity
- Click SELL button
Expected: Item removed from inventory, gold added

**Step 4: Verify sell price calculation**

- Check that sell price = base_price × 0.5 (buy_multiplier)
Expected: Selling gives 50% of item's base price

**Step 5: Verify empty state**

- Sell all sellable items
Expected: "No sellable items" message appears

**Step 6: Document results**

Add to: `docs/testing/2026-01-12-shop-buy-test.md`

---

## Task 19: Test Stock Restock System

**Files:**
- None (manual testing)

**Step 1: Buy items to reduce stock**

- Buy 2 iron swords (leaving 1 in stock)
Expected: Stock shows x1

**Step 2: Wait for restock (5 minutes)**

- Wait 5 minutes of server uptime
Expected: Stock increases by restock_rate (1), now shows x2

**Step 3: Verify gradual restock**

- Wait another 5 minutes
Expected: Stock increases again to x3 (max_quantity)

**Step 4: Verify max stock limit**

- Wait another 5 minutes
Expected: Stock stays at x3 (doesn't exceed max)

**Step 5: Verify restock updates for other players**

- Have two clients connected
- One buys item, other watches stock
Expected: Both see stock decrease immediately

**Step 6: Document results**

Add to: `docs/testing/2026-01-12-shop-buy-test.md`

---

## Task 20: Polish and Edge Cases

**Files:**
- Modify: `rust-server/src/game.rs` (add validation checks)
- Modify: `client/src/render/ui/shop.rs` (improve UI feedback)

**Step 1: Add validation for zero/negative quantities**

In `rust-server/src/game.rs`, in handle_shop_buy and handle_shop_sell, add at start:

```rust
if quantity <= 0 {
    self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Invalid quantity".to_string()));
    return;
}
```

**Step 2: Add NPC death handling**

In client, when shop is open and NPC dies:

```rust
// In client/src/network/client.rs, in NPC death/removal handler
if let Some(shop_npc_id) = &game_state.ui_state.shop_npc_id {
    if shop_npc_id == &npc_id {
        game_state.ui_state.crafting_open = false;
        game_state.ui_state.shop_data = None;
        game_state.add_chat_message("System", "Merchant has died!");
    }
}
```

**Step 3: Add proper item sprite rendering**

In `client/src/render/ui/shop.rs`, replace placeholder rectangles with actual item sprites:

```rust
// Instead of: draw_rectangle(item_x + 10.0, item_y + 10.0, 60.0, 40.0, WHITE);
if let Some(item_def) = game_state.item_registry.get(&stock_item.item_id) {
    // Render actual sprite using item_def.sprite
    // This depends on your sprite rendering system
}
```

**Step 4: Add loading states**

Handle case where shop_data might not be loaded yet:

```rust
// In render_shop_tab
let shop_data = match &game_state.ui_state.shop_data {
    Some(data) => data,
    None => {
        draw_text("Loading shop...", panel_x + 200.0, panel_y + 200.0, 20.0, WHITE);
        return;
    }
};
```

**Step 5: Verify compilation**

Run: `cd rust-server && cargo check && cd ../client && cargo check`
Expected: Both compile successfully

**Step 6: Commit**

```bash
git add rust-server/src/game.rs client/src/render/ui/shop.rs client/src/network/client.rs
git commit -m "feat: add shop polish and edge case handling

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 21: Create Additional Shop Definitions

**Files:**
- Create: `rust-server/data/shops/general_store.toml`
- Create: `rust-server/data/shops/alchemist.toml`

**Step 1: Create general store shop**

Create `rust-server/data/shops/general_store.toml`:

```toml
id = "general_store"
display_name = "General Goods"

[[stock]]
item_id = "health_potion"
max_quantity = 10
restock_rate = 2

[[stock]]
item_id = "bread"
max_quantity = 20
restock_rate = 5

[[stock]]
item_id = "rope"
max_quantity = 5
restock_rate = 1
```

**Step 2: Create alchemist shop**

Create `rust-server/data/shops/alchemist.toml`:

```toml
id = "alchemist"
display_name = "Alchemist's Potions"

[[stock]]
item_id = "health_potion"
max_quantity = 15
restock_rate = 3

[[stock]]
item_id = "mana_potion"
max_quantity = 15
restock_rate = 3

[[stock]]
item_id = "antidote"
max_quantity = 8
restock_rate = 2
```

**Step 3: Verify shops load**

Run: `cd rust-server && cargo run`
Expected: "Loaded 3 shops" in console

**Step 4: Commit**

```bash
git add rust-server/data/shops/
git commit -m "feat: add general store and alchemist shops

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 22: Documentation

**Files:**
- Create: `docs/features/shop-system.md`

**Step 1: Create feature documentation**

Create `docs/features/shop-system.md`:

```markdown
# Shop System

## Overview

The shop system allows players to buy and sell items with merchant NPCs. Merchants have limited stock that gradually restocks over time.

## Architecture

### Data Flow
1. Player interacts with merchant NPC
2. Server sends ShopData with current stock and prices
3. Client renders shop UI with Buy/Sell tabs
4. Player selects items and confirms transaction
5. Server validates and executes transaction
6. Server broadcasts stock updates to all players viewing the shop

### Components

**Server:**
- `ShopRegistry` - Loads and manages shop definitions
- `ShopDefinition` - Defines shop stock and restock rates
- Shop handlers - Validate and execute buy/sell transactions
- Restock system - Periodically increases stock up to max

**Client:**
- Shop UI module - Renders merchant interface with tabs
- Shop state - Tracks open shop, selected items, quantities
- Input handlers - Process clicks and keyboard shortcuts

## Shop Definition Format

```toml
id = "shop_name"
display_name = "Display Name"

[[stock]]
item_id = "item_id"
max_quantity = 10
restock_rate = 2  # items added per restock interval
```

## Merchant Configuration

In entity TOML files:

```toml
[npc_name.merchant]
shop_id = "shop_name"
buy_multiplier = 0.5   # merchant pays 50% when buying from player
sell_multiplier = 1.0  # merchant charges 100% when selling to player
restock_interval_minutes = 5
```

## UI Controls

- **Q/E or Click**: Switch between Shop and Recipes tabs
- **Tab or Click**: Switch between Buy and Sell sub-tabs
- **Click item**: Select item for transaction
- **+/- or Click**: Adjust quantity
- **Enter or Click**: Confirm transaction
- **Escape**: Close shop

## Price Calculation

- **Buy from merchant**: base_price × sell_multiplier
- **Sell to merchant**: base_price × buy_multiplier

## Network Protocol

**Client → Server:**
- `ShopBuy { npc_id, item_id, quantity }`
- `ShopSell { npc_id, item_id, quantity }`

**Server → Client:**
- `ShopData { npc_id, shop }` - Sent when shop opens
- `ShopResult { success, action, item_id, quantity, gold_change, error }`
- `ShopStockUpdate { npc_id, item_id, new_quantity }` - Broadcast on stock change

## Validation

**Buy transaction validates:**
- Player exists and is alive
- NPC exists and is alive
- Player within interaction range (2.5 tiles)
- Item in stock with sufficient quantity
- Player has enough gold
- Player has inventory space

**Sell transaction validates:**
- Player exists and is alive
- NPC exists and is alive
- Player within interaction range
- Item is sellable
- Player has sufficient quantity

## Restock System

- Runs every minute on server
- For shops with `restock_interval_minutes` set
- Adds `restock_rate` to each item's current stock
- Stock capped at `max_quantity`
- Broadcasts updates to players viewing shop
```

**Step 2: Commit**

```bash
git add docs/features/shop-system.md
git commit -m "docs: add shop system documentation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Completion Checklist

- [ ] Server shop structures and registry
- [ ] Shop protocol messages
- [ ] Server buy/sell handlers
- [ ] Server restock system
- [ ] Client shop data structures
- [ ] Client message handlers
- [ ] Shop UI rendering
- [ ] Shop input handling
- [ ] Main tab switching (Shop ↔ Recipes)
- [ ] Sub-tab switching (Buy ↔ Sell)
- [ ] Merchant entity configuration
- [ ] Manual testing (buy, sell, restock)
- [ ] Edge case handling
- [ ] Additional shop definitions
- [ ] Documentation

---

## Notes

- **DRY**: Shop system reuses existing patterns from crafting (UI layout, message protocol, registry loading)
- **YAGNI**: No unnecessary features like merchant reputation, dynamic pricing, or trade quests
- **TDD**: Manual testing approach due to UI-heavy implementation; consider adding integration tests later
- **Commits**: Frequent small commits after each logical component
