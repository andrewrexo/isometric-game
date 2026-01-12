# Shop System Documentation

## Overview

The shop system enables players to buy and sell items with merchant NPCs. It features a client-side UI with buy/sell tabs, server-side inventory and stock management, real-time price calculation based on merchant markup, and automatic stock restocking over time.

**Key Features:**
- Buy items from merchant stock with configurable markup
- Sell items from player inventory at configurable rates
- Real-time stock tracking and restocking
- Distance validation (must be within 2.5 tiles)
- Transaction validation (gold, inventory space, stock availability)
- Quantity adjustment controls (+1, +10, -1, -10, Max)
- Two-tab UI (Buy/Sell) with item previews

## Architecture

### Data Flow

```
Client                          Server                          Data Files
------                          ------                          ----------
Player clicks NPC    ->    ShopOpen message        ->      shops/*.toml
                                                           entities/npcs/*.toml

                      <-    ShopData message        <-     ShopDefinition
                                                           MerchantConfig

Player clicks Buy    ->    ShopBuy message         ->     Validate transaction
                                                           Update gold
                                                           Update stock
                                                           Update inventory

                      <-    ShopResult message      <-     Success/failure
                           InventorySync message

Player clicks Sell   ->    ShopSell message        ->     Validate transaction
                                                           Update gold
                                                           Update inventory
                                                           (no stock change)

                      <-    ShopResult message      <-     Success/failure
                           InventorySync message
```

### Components

#### Server Components

1. **ShopRegistry** (`rust-server/src/shop.rs`)
   - Loads shop definitions from `data/shops/*.toml`
   - Tracks current stock for all shops
   - Handles restocking logic via periodic timer
   - Thread-safe access via Arc<RwLock>

2. **MerchantConfig** (part of EntityPrototype)
   - Links NPC to shop via `shop_id`
   - Defines buy/sell multipliers
   - Configures restock interval

3. **Game Loop Integration** (`rust-server/src/game.rs`)
   - `handle_shop_open()` - validates NPC, loads shop data
   - `handle_shop_buy()` - validates transaction, updates stock/inventory/gold
   - `handle_shop_sell()` - validates transaction, updates inventory/gold
   - Periodic restock task runs every minute

#### Client Components

1. **Shop UI** (`client/src/render/ui/shop.rs`)
   - Two-tab interface (Buy/Sell)
   - Item list with icons, names, prices, stock
   - Transaction bar with quantity controls
   - Real-time total price calculation

2. **Network Client** (`client/src/network/client.rs`)
   - Sends shop interaction messages
   - Receives shop data and results
   - Handles edge cases (NPC death, distance)

3. **Game State** (`client/src/game.rs`)
   - Stores active shop data
   - Tracks selected items and quantities
   - Manages shop UI state

## Shop Definition Format

Shop definitions are stored in `rust-server/data/shops/*.toml`:

```toml
id = "blacksmith"
display_name = "Blacksmith's Forge"

[[stock]]
item_id = "iron_sword"
max_quantity = 5
restock_rate = 1

[[stock]]
item_id = "steel_armor"
max_quantity = 3
restock_rate = 1

[[stock]]
item_id = "health_potion"
max_quantity = 10
restock_rate = 2
```

### Fields

- `id`: Unique shop identifier (referenced by merchant NPCs)
- `display_name`: Human-readable shop name shown in UI
- `stock`: Array of items available for purchase
  - `item_id`: Must match item definition in items registry
  - `max_quantity`: Maximum stock (restocking target)
  - `restock_rate`: Items added per restock interval (default: 1/minute)

### Stock Behavior

- Initial stock starts at `max_quantity`
- When items are purchased, current stock decreases
- Restock timer adds `restock_rate` items every interval (capped at `max_quantity`)
- Stock is shared across all players on the server

## Merchant Configuration

Merchant NPCs are configured in `rust-server/data/entities/npcs/*.toml`:

```toml
[blacksmith]
display_name = "Blacksmith"
sprite = "blacksmith"
animation_type = "humanoid"
description = "The village blacksmith. He can repair and improve equipment."

[blacksmith.behaviors]
hostile = false
merchant = true

[blacksmith.merchant]
shop_id = "blacksmith"
buy_multiplier = 0.5
sell_multiplier = 1.0
restock_interval_minutes = 5
```

### Merchant Fields

- `shop_id`: Links to shop definition (must exist in shops registry)
- `buy_multiplier`: Player selling price (0.5 = player gets 50% of base price)
- `sell_multiplier`: Player buying price (1.0 = player pays 100% of base price)
- `restock_interval_minutes`: How often stock replenishes (in minutes)

### Price Examples

If an item has base value of 100 gold:
- Player buys for: 100 × 1.0 = 100 gold (sell_multiplier)
- Player sells for: 100 × 0.5 = 50 gold (buy_multiplier)

## UI Controls

### Opening a Shop

1. Player must be within 2.5 tiles of merchant NPC
2. Click on merchant NPC
3. Press 'F' to interact
4. Shop UI opens in crafting panel

### Buy Tab

- Displays merchant's stock items
- Shows item icon, name, price, and current stock
- Click item to select
- Use quantity controls: -10, -1, +1, +10, Max
- Max button sets quantity to min(stock, affordable_quantity)
- Click "Buy" button to purchase
- Transaction validates gold and inventory space

### Sell Tab

- Displays player's inventory items
- Shows item icon, name, and sell price
- Click item to select
- Use quantity controls: -10, -1, +1, +10, Max
- Max button sets quantity to item stack size
- Click "Sell" button to complete transaction
- Cannot sell equipped items

### Keyboard Shortcuts

- `Tab`: Switch between Buy/Sell tabs
- `Escape`: Close shop UI
- `F`: Re-open shop (if still in range)

## Price Calculation

### Buying (Player purchasing from merchant)

```rust
final_price = item.value × merchant.sell_multiplier × quantity
```

Example:
- Iron Sword base value: 150 gold
- Merchant sell_multiplier: 1.2
- Quantity: 2
- Final price: 150 × 1.2 × 2 = 360 gold

### Selling (Player selling to merchant)

```rust
final_price = item.value × merchant.buy_multiplier × quantity
```

Example:
- Iron Ore base value: 20 gold
- Merchant buy_multiplier: 0.5
- Quantity: 10
- Final price: 20 × 0.5 × 10 = 100 gold

## Network Protocol

### Messages

#### Client → Server

**ShopOpen**
```json
{
  "type": "shopOpen",
  "npc_id": "npc_uuid"
}
```

**ShopBuy**
```json
{
  "type": "shopBuy",
  "npc_id": "npc_uuid",
  "item_id": "iron_sword",
  "quantity": 2
}
```

**ShopSell**
```json
{
  "type": "shopSell",
  "npc_id": "npc_uuid",
  "item_id": "iron_ore",
  "quantity": 10
}
```

#### Server → Client

**ShopData**
```json
{
  "type": "shopData",
  "shop_id": "blacksmith",
  "display_name": "Blacksmith's Forge",
  "stock": [
    {
      "item_id": "iron_sword",
      "current_quantity": 3,
      "max_quantity": 5,
      "buy_price": 180,
      "sell_price": 75
    }
  ]
}
```

**ShopResult**
```json
{
  "type": "shopResult",
  "success": true,
  "action": "buy",
  "item_id": "iron_sword",
  "quantity": 2,
  "total_cost": 360,
  "error": null
}
```

## Validation

### Shop Open Validation

- Player must be within 2.5 tiles of NPC
- NPC must be alive
- NPC must have merchant behavior enabled
- Shop definition must exist in registry
- Player must be active and alive

### Buy Transaction Validation

1. **Distance Check**: Player within 2.5 tiles
2. **Quantity Check**: `quantity > 0`
3. **Stock Check**: `quantity <= current_stock`
4. **Gold Check**: `player.gold >= total_cost`
5. **Inventory Space**: Player has room for items
6. **Item Exists**: Item definition exists in registry

Failure scenarios:
- "Invalid quantity" - zero or negative
- "Not enough stock" - insufficient merchant inventory
- "Not enough gold" - player cannot afford
- "Inventory full" - no space for items
- "Too far from merchant" - player moved away

### Sell Transaction Validation

1. **Distance Check**: Player within 2.5 tiles
2. **Quantity Check**: `quantity > 0`
3. **Inventory Check**: Player owns sufficient quantity
4. **Item Check**: Item is not equipped
5. **Price Check**: Item has valid base value

Failure scenarios:
- "Invalid quantity" - zero or negative
- "Not enough items" - insufficient player inventory
- "Cannot sell equipped items" - item currently equipped
- "Too far from merchant" - player moved away

## Restock System

### Restock Logic

Runs periodically (every minute) in the game loop:

```rust
for shop in shops {
    let interval_ms = merchant.restock_interval_minutes * 60 * 1000;
    if time_since_last_restock >= interval_ms {
        for stock_item in shop.stock {
            let missing = stock_item.max_quantity - stock_item.current_quantity;
            let to_add = min(missing, stock_item.restock_rate);
            stock_item.current_quantity += to_add;
        }
        shop.last_restock_time = now;
    }
}
```

### Restock Rate Examples

- `restock_rate = 1`: Adds 1 item per interval
- `restock_rate = 3`: Adds 3 items per interval
- `restock_interval_minutes = 5`: Restock every 5 minutes

If blacksmith has:
- `max_quantity = 10`
- `current_quantity = 3`
- `restock_rate = 2`
- `restock_interval_minutes = 5`

Then every 5 minutes, stock increases by 2 (until max of 10).

## Edge Cases

### NPC Death

When a merchant NPC dies:
- Shop UI closes immediately
- `shop_data` and `shop_npc_id` cleared
- Player receives no error message
- Shop can be reopened when NPC respawns

### Distance Check

- Shop opens only within 2.5 tiles
- Transactions fail if player moves too far
- Error message: "Too far from merchant"
- Player must move back within range to continue

### Concurrent Transactions

- Shop stock is server-authoritative
- Multiple players can race to buy last item
- First transaction to reach server wins
- Second player receives "Not enough stock" error

### Inventory Management

When buying:
- Items stack with existing inventory stacks
- New stacks created if needed
- Transaction fails if no slots available
- Partial purchases not supported (all-or-nothing)

When selling:
- Items removed from inventory
- Equipped items cannot be sold
- Stacks split automatically if selling partial amount

## Implementation Checklist

- [x] Server shop registry and definitions
- [x] Server merchant configuration
- [x] Server shop open handler
- [x] Server shop buy handler
- [x] Server shop sell handler
- [x] Server restock system
- [x] Client shop UI (buy/sell tabs)
- [x] Client quantity controls
- [x] Client-server protocol messages
- [x] Distance validation
- [x] Transaction validation
- [x] Edge case handling (NPC death, distance)
- [x] Price calculation and display
- [x] Shop definitions (blacksmith, general_store, alchemist)
- [x] Merchant NPC configuration

## Future Enhancements

Potential improvements for future iterations:

1. **Dynamic Pricing**
   - Supply/demand adjustments
   - Reputation-based discounts
   - Bulk purchase discounts

2. **Special Orders**
   - NPC requests specific items
   - Time-limited quests
   - Bonus rewards

3. **Shop Upgrades**
   - Unlock new items via quests
   - Increase max stock
   - Reduce restock time

4. **Trade History**
   - Track purchase/sale history
   - Show price trends
   - Player statistics

5. **Multiple Currencies**
   - Special tokens or gems
   - Barter system
   - Currency exchange

6. **Limited-Time Sales**
   - Daily deals
   - Seasonal items
   - Flash sales
