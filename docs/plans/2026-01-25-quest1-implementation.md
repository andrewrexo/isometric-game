# Quest 1 "What Happened Here" Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the first quest end-to-end as a vertical slice of the early game progression system.

**Architecture:** Add new content via TOML data files (items, monsters, NPCs, quests). Entity spawns are placed via the mapper tool in Tiled JSON format. Quest system already exists and supports kill/collect objectives.

**Tech Stack:** Rust server with TOML data files, Tiled JSON maps

---

## Task 1: Add Tier 0 Starting Equipment

**Files:**
- Modify: `rust-server/data/items/equipment.toml`

**Step 1: Add Worn Pitchfork (Tier 0 weapon)**

Add at the end of the WEAPONS section:

```toml
[worn_pitchfork]
display_name = "Worn Pitchfork"
sprite = "worn_pitchfork"
description = "A rusty farm tool. Better than nothing."
category = "equipment"
max_stack = 1
base_price = 10
sellable = true
[worn_pitchfork.equipment]
slot_type = "weapon"
attack_level_required = 1
attack_bonus = 2
strength_bonus = 3
defence_bonus = 0
```

**Step 2: Add Torn Clothes (Tier 0 body)**

Add at the end of the BODY ARMOR section:

```toml
[torn_clothes]
display_name = "Torn Clothes"
sprite = "torn_clothes"
description = "Ragged clothing barely holding together."
category = "equipment"
max_stack = 1
base_price = 5
sellable = true
[torn_clothes.equipment]
slot_type = "body"
defence_level_required = 1
attack_bonus = 0
strength_bonus = 0
defence_bonus = 1
```

**Step 3: Add Worn Sandals (Tier 0 feet)**

Add at the end of the FEET section:

```toml
[worn_sandals]
display_name = "Worn Sandals"
sprite = "worn_sandals"
description = "Flimsy sandals that barely protect your feet."
category = "equipment"
max_stack = 1
base_price = 5
sellable = true
[worn_sandals.equipment]
slot_type = "feet"
defence_level_required = 1
attack_bonus = 0
strength_bonus = 0
defence_bonus = 0
```

**Step 4: Commit**

```bash
git add rust-server/data/items/equipment.toml
git commit -m "feat(items): add Tier 0 starting equipment (worn pitchfork, torn clothes, worn sandals)"
```

---

## Task 2: Add Tier 1 Quest Reward Equipment

**Files:**
- Modify: `rust-server/data/items/equipment.toml`

**Step 1: Add Salvaged Sword (Tier 1 weapon - Quest 1 reward)**

Add after worn_pitchfork in the WEAPONS section:

```toml
[salvaged_sword]
display_name = "Salvaged Sword"
sprite = "salvaged_sword"
description = "A battered but functional blade recovered from the ruins."
category = "equipment"
max_stack = 1
base_price = 75
sellable = true
[salvaged_sword.equipment]
slot_type = "weapon"
attack_level_required = 1
attack_bonus = 6
strength_bonus = 8
defence_bonus = 0
```

**Step 2: Add Salvaged Tunic (Tier 1 body - drop)**

Add after torn_clothes in the BODY ARMOR section:

```toml
[salvaged_tunic]
display_name = "Salvaged Tunic"
sprite = "salvaged_tunic"
description = "A scavenged tunic. Stained but sturdy."
category = "equipment"
max_stack = 1
base_price = 60
sellable = true
[salvaged_tunic.equipment]
slot_type = "body"
defence_level_required = 1
attack_bonus = 0
strength_bonus = 0
defence_bonus = 5
```

**Step 3: Commit**

```bash
git add rust-server/data/items/equipment.toml
git commit -m "feat(items): add Tier 1 salvaged equipment (sword, tunic)"
```

---

## Task 3: Add Quest Materials

**Files:**
- Modify: `rust-server/data/items/materials.toml`

**Step 1: Add Spoiled Meat (quest objective item)**

Add at the end of the Monster Drops section:

```toml
[spoiled_meat]
display_name = "Spoiled Meat"
sprite = "spoiled_meat"
description = "Tainted meat from a corrupted pig. Inedible but proof of the kill."
category = "material"
max_stack = 99
base_price = 3
```

**Step 2: Add Tainted Hide (crafting material)**

```toml
[tainted_hide]
display_name = "Tainted Hide"
sprite = "tainted_hide"
description = "Corrupted pig hide. Can be used for basic crafting."
category = "material"
max_stack = 99
base_price = 5
```

**Step 3: Commit**

```bash
git add rust-server/data/items/materials.toml
git commit -m "feat(items): add corrupted pig drop materials (spoiled meat, tainted hide)"
```

---

## Task 4: Create Corrupted Pig Monster

**Files:**
- Create: `rust-server/data/entities/monsters/corrupted_creatures.toml`

**Step 1: Create the corrupted creatures file with Corrupted Pig**

```toml
# Corrupted Creatures - Twisted by the corruption spreading across the land

# =============================================================================
# Tier 1 - Farm Creatures (Combat 3-10)
# =============================================================================

[corrupted_pig]
display_name = "Corrupted Pig"
sprite = "corrupted_pig"
animation_type = "standard"
description = "A farm pig twisted by the corruption. Its eyes glow with malice."

[corrupted_pig.stats]
level = 1
max_hp = 20
damage = 2
attack_range = 1
aggro_range = 5
chase_range = 5
move_cooldown_ms = 600
attack_cooldown_ms = 2000
respawn_time_ms = 30000

[corrupted_pig.rewards]
exp_base = 35
gold_min = 3
gold_max = 8

[[corrupted_pig.loot]]
item_id = "spoiled_meat"
drop_chance = 0.80
quantity_min = 1
quantity_max = 1

[[corrupted_pig.loot]]
item_id = "tainted_hide"
drop_chance = 0.30
quantity_min = 1
quantity_max = 2

[[corrupted_pig.loot]]
item_id = "salvaged_tunic"
drop_chance = 0.05
quantity_min = 1
quantity_max = 1

[corrupted_pig.behaviors]
hostile = true
wander_enabled = true
wander_radius = 4
wander_pause_min_ms = 2000
wander_pause_max_ms = 5000
```

**Step 2: Commit**

```bash
git add rust-server/data/entities/monsters/corrupted_creatures.toml
git commit -m "feat(entities): add Corrupted Pig monster for Quest 1"
```

---

## Task 5: Create Elder Mara NPC

**Files:**
- Modify: `rust-server/data/entities/npcs/villagers.toml`

**Step 1: Add Elder Mara (rename/update existing elder_villager)**

Replace the existing `elder_villager` section with Elder Mara, keeping the same ID for backwards compatibility:

```toml
# ============================================================================
# Elder Mara - Cursed Lands Quest Giver (Ruined Village)
# ============================================================================
[elder_villager]
display_name = "Elder Mara"
sprite = "village_elder"
animation_type = "humanoid"
description = "The weary elder of the ruined village. She carries the weight of her people's suffering."

[elder_villager.stats]
max_hp = 200
damage = 0
attack_range = 0
aggro_range = 0
chase_range = 0
move_cooldown_ms = 0
attack_cooldown_ms = 0
respawn_time_ms = 0

[elder_villager.rewards]
exp_base = 0
gold_min = 0
gold_max = 0

[elder_villager.behaviors]
hostile = false
quest_giver = true

[elder_villager.quest_giver]
available_quests = ["what_happened_here", "spreading_rot"]

[elder_villager.dialogue]
greeting = "You survived... thank the light. The corruption came without warning. Our village lies in ruins."
quest_available = "I have a task that might help us understand what happened here."
quest_complete = "You've done well. Perhaps there is hope for us yet."
```

**Step 2: Commit**

```bash
git add rust-server/data/entities/npcs/villagers.toml
git commit -m "feat(npcs): update Village Elder to Elder Mara with new dialogue"
```

---

## Task 6: Create Quest 1 "What Happened Here"

**Files:**
- Create: `rust-server/data/quests/cursed_lands/what_happened_here.toml`

**Step 1: Create the quests directory and quest file**

```bash
mkdir -p rust-server/data/quests/cursed_lands
```

**Step 2: Create the quest definition**

```toml
# What Happened Here - First quest in the Cursed Lands chain
# Given by Elder Mara in the Ruined Village

[quest]
id = "what_happened_here"
name = "What Happened Here"
description = "The corruption has twisted the farm animals into hostile creatures. Elder Mara needs you to investigate and thin their numbers."
giver_npc = "elder_villager"
level_required = 1
repeatable = false

[quest.chain]
next = "spreading_rot"

[[quest.objectives]]
id = "kill_corrupted_pigs"
type = "kill_monster"
target = "corrupted_pig"
count = 15
description = "Slay 15 Corrupted Pigs"

[[quest.objectives]]
id = "collect_spoiled_meat"
type = "collect_item"
target = "spoiled_meat"
count = 10
description = "Collect 10 Spoiled Meat"

[[quest.objectives]]
id = "return_to_elder"
type = "talk_to"
target = "elder_villager"
description = "Return to Elder Mara"
sequential = true

[quest.rewards]
exp = 150
gold = 75
items = [
    { id = "salvaged_sword", count = 1 }
]

[quest.dialogue]
offer = "The pigs... they've changed. Their eyes glow with that terrible corruption. They attack anyone who comes near the old farms. We need to know how deep this sickness runs. Can you investigate?"
accept = "Be careful out there. Kill the corrupted pigs and bring me proof - their tainted meat will tell us much about this corruption. Return when you've gathered enough evidence."
progress = "Have you learned anything about the corruption? The village grows more fearful each day."
complete = "This meat... the corruption runs deep. But you've proven yourself capable. Take this blade - I salvaged it from the ruins. You'll need it for what lies ahead."
```

**Step 3: Commit**

```bash
git add rust-server/data/quests/cursed_lands/what_happened_here.toml
git commit -m "feat(quests): add Quest 1 'What Happened Here' for Cursed Lands chain"
```

---

## Task 7: Update Shop Inventory

**Files:**
- Modify: `rust-server/data/shops/blacksmith.toml`

**Step 1: Update blacksmith shop with Tier 0-1 gear**

Replace the contents with:

```toml
id = "blacksmith"
display_name = "Blacksmith's Wares"

# Tier 0 - Starting gear replacements
[[stock]]
item_id = "worn_pitchfork"
max_quantity = 5
restock_rate = 1

[[stock]]
item_id = "torn_clothes"
max_quantity = 5
restock_rate = 1

[[stock]]
item_id = "worn_sandals"
max_quantity = 5
restock_rate = 1

# Tier 1 - Salvaged gear (fallback for unlucky drops)
[[stock]]
item_id = "salvaged_sword"
max_quantity = 2
restock_rate = 1

[[stock]]
item_id = "salvaged_tunic"
max_quantity = 3
restock_rate = 1

# Existing items
[[stock]]
item_id = "goblin_spear"
max_quantity = 3
restock_rate = 1

[[stock]]
item_id = "peasant_suit"
max_quantity = 10
restock_rate = 1

[[stock]]
item_id = "training_gloves"
max_quantity = 10
restock_rate = 1

[[stock]]
item_id = "peasant_boots"
max_quantity = 4
restock_rate = 1
```

**Step 2: Commit**

```bash
git add rust-server/data/shops/blacksmith.toml
git commit -m "feat(shops): update blacksmith with Tier 0-1 equipment"
```

---

## Task 8: Add Corrupted Pig Spawns to Map

**Files:**
- Modify: `mapper/public/maps/chunk_0_0.json` (via Tiled editor or manual JSON edit)
- Modify: `mapper/public/maps/chunk_-1_0.json` (via Tiled editor or manual JSON edit)

**Note:** Entity spawns are added via the "entities" layer in Tiled. Each entity object needs:
- `entity_id` property (string): "corrupted_pig"
- `level` property (int): 1
- Position (x, y in pixels, will be converted to grid)

**Step 1: Add ~10 Corrupted Pig spawns to chunk_0_0.json**

In the "entities" layer objects array, add entries like:

```json
{
    "height": 32,
    "id": 100,
    "name": "Corrupted Pig",
    "properties": [
        {
            "name": "entity_id",
            "type": "string",
            "value": "corrupted_pig"
        },
        {
            "name": "level",
            "type": "int",
            "value": 1
        }
    ],
    "rotation": 0,
    "type": "",
    "visible": true,
    "width": 32,
    "x": 640,
    "y": 320
}
```

Add approximately 10 spawns spread across the map at various positions.

**Step 2: Add ~8 Corrupted Pig spawns to chunk_-1_0.json**

Similar process for the adjacent chunk.

**Step 3: Commit**

```bash
git add mapper/public/maps/chunk_0_0.json mapper/public/maps/chunk_-1_0.json
git commit -m "feat(maps): add Corrupted Pig spawns to starting chunks"
```

---

## Task 9: Test the Implementation

**Step 1: Start the server**

```bash
cd rust-server && cargo run
```

**Step 2: Verify entity registry loads new content**

Check server logs for:
- "Loaded entity prototype: corrupted_pig"
- "Loaded quest: what_happened_here"
- "Loaded item: spoiled_meat"

**Step 3: In-game testing checklist**

- [ ] Talk to Elder Mara, accept quest
- [ ] Kill Corrupted Pigs, verify XP gain (~35 per kill)
- [ ] Verify Spoiled Meat drops (~80% rate)
- [ ] Verify Tainted Hide drops (~30% rate)
- [ ] Verify Salvaged Tunic rare drop (~5% rate)
- [ ] Collect 10 Spoiled Meat
- [ ] Return to Elder Mara
- [ ] Complete quest, receive Salvaged Sword + 75 gold + 150 XP
- [ ] Verify sword equips and provides +6 attack, +8 strength

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: address issues found during Quest 1 testing"
```

---

## Summary

After completing all tasks, Quest 1 "What Happened Here" will be fully playable:

1. **Starting gear**: Worn Pitchfork, Torn Clothes, Worn Sandals (Tier 0)
2. **Monster**: Corrupted Pig (20 HP, 35 XP, drops Spoiled Meat + Tainted Hide)
3. **Quest**: Kill 15 pigs, collect 10 meat, return to Elder Mara
4. **Reward**: Salvaged Sword (Tier 1), 75 gold, 150 XP
5. **Shops**: Updated with Tier 0-1 gear as fallback

Total files created/modified:
- `rust-server/data/items/equipment.toml` (modified)
- `rust-server/data/items/materials.toml` (modified)
- `rust-server/data/entities/monsters/corrupted_creatures.toml` (created)
- `rust-server/data/entities/npcs/villagers.toml` (modified)
- `rust-server/data/quests/cursed_lands/what_happened_here.toml` (created)
- `rust-server/data/shops/blacksmith.toml` (modified)
- `mapper/public/maps/chunk_0_0.json` (modified)
- `mapper/public/maps/chunk_-1_0.json` (modified)
