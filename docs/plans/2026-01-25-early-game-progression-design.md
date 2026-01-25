# Early Game Progression Design

## Overview

### Theme: The Cursed Lands

A once-peaceful village has been struck by a mysterious corruption. The player is a **local survivor** who wakes up after the catastrophe, their home in ruins. Familiar farm animals are now twisted and aggressive. The forest has become dangerous. And worst of all - some former neighbors didn't escape the corruption's grip.

### Progression Philosophy

- **4 gear tiers**: Worn (starter) → Salvaged (drops) → Restored (quests/crafting) → Purified (boss/quest capstone)
- **5 equipment slots**: Weapon, Body, Boots, Helm, Ring (Gloves as quest chain reward)
- **Linear progression**: Each tier is strictly better than the last
- **8 quests** guiding ~4 hours of progression, plus ~2 hours of boss farming
- **Level targets**: Players finish around Combat 35, Hitpoints 38

### Core Loop

1. Accept quest from Village Elder
2. Fight corrupted enemies, collect materials
3. Gain XP and gold naturally through kills
4. Complete quest for gear upgrade
5. Repeat with harder enemies
6. Farm boss for Tier 3 drops and bow parts
7. Craft the Purified Longbow as graduation prize

---

## Quest Chain & NPCs

### Quest Givers by Location

| Location | NPC | Quests Given | Fantasy |
|----------|-----|--------------|---------|
| Ruined Village | Elder Mara | Quests 1-2 | Survivor leader, sends you to investigate |
| Abandoned Farm | Farmer Jorik | Quest 3 | Hiding in his barn, needs the farms cleared |
| Forest Edge Camp | Scout Alara | Quests 4-5 | Ranger tracking the corruption's spread |
| Fallen Outpost | Captain Roderick | Quests 6-7 | Last surviving militia officer, grim duty |
| Corrupted Chapel | Spirit of Father Aldric | Quest 8 | Ghost of the village priest, guides you to face The First Fallen |

### Quest Chain Flow

```
Ruined Village (Elder Mara)
    ├── Quest 1: "What Happened Here"
    └── Quest 2: "The Spreading Rot"
            ↓
Abandoned Farm (Farmer Jorik)
    └── Quest 3: "Clearing the Farms"
            ↓
Forest Edge Camp (Scout Alara)
    ├── Quest 4: "Into the Woods"
    └── Quest 5: "The Pack Returns"
            ↓
Fallen Outpost (Captain Roderick)
    ├── Quest 6: "Faces We Knew"
    └── Quest 7: "Hold the Line"
            ↓
Corrupted Chapel (Spirit of Father Aldric)
    └── Quest 8: "The First Fallen"
```

### Dialogue Tone by NPC

- **Elder Mara** - Weary but hopeful, grateful for help
- **Scout Alara** - Professional, tactical, warns of what's ahead
- **Farmer Jorik** - Scared, practical, just wants his land back
- **Captain Roderick** - Haunted, duty-bound, struggles with fighting former comrades
- **Spirit of Father Aldric** - Sorrowful, peaceful, seeks release for The First Fallen

---

## Detailed Quest Breakdown

| # | Quest | Giver | Objectives | Rewards |
|---|-------|-------|------------|---------|
| 1 | "What Happened Here" | Elder Mara | Kill 15 Corrupted Pigs, collect 10 Spoiled Meat | 75g, 150 XP, **Salvaged Sword** |
| 2 | "The Spreading Rot" | Elder Mara | Kill 20 Blighted Slimes, collect 15 Slime Cores | 100g, 200 XP, **Salvaged Band** (ring) |
| 3 | "Clearing the Farms" | Farmer Jorik | Kill 25 Corrupted Pigs, Kill 15 Blighted Slimes | 125g, 250 XP, **Salvaged Boots** |
| 4 | "Into the Woods" | Scout Alara | Kill 20 Shadow Spiders, collect 20 Tainted Webbing | 150g, 300 XP, **Restored Blade** |
| 5 | "The Pack Returns" | Scout Alara | Kill 25 Rotting Wolves, collect 15 Cursed Fangs | 175g, 350 XP, **Restored Signet** (ring) |
| 6 | "Faces We Knew" | Captain Roderick | Kill 30 Hollow Villagers, collect 20 Tattered Cloth | 200g, 400 XP, **Restored Chainmail** |
| 7 | "Hold the Line" | Captain Roderick | Kill 30 Corrupted Militia, recover 5 Militia Badges | 250g, 500 XP, **Restored Greaves** |
| 8 | "The First Fallen" | Spirit of Father Aldric | Defeat The First Fallen 3 times | 500g, 750 XP, **Purified Gauntlets** |

---

## Monster Roster & Stats

### Tier 1 - Farm Creatures (Combat 3-10)

| Monster | HP | Damage | XP | Gold | Respawn | Aggro Range |
|---------|-----|--------|-----|------|---------|-------------|
| Corrupted Pig | 20 | 2 | 35 | 3-8 | 30s | 5 tiles |
| Blighted Slime | 25 | 3 | 45 | 4-10 | 30s | 4 tiles |

**Location**: Around Ruined Village and Abandoned Farm

### Tier 2 - Forest Creatures (Combat 10-20)

| Monster | HP | Damage | XP | Gold | Respawn | Aggro Range |
|---------|-----|--------|-----|------|---------|-------------|
| Shadow Spider | 40 | 5 | 80 | 8-15 | 45s | 6 tiles |
| Rotting Wolf | 55 | 7 | 110 | 10-20 | 45s | 7 tiles |

**Location**: Forest between Farm and Forest Edge Camp

### Tier 3 - Corrupted Humans (Combat 20-30)

| Monster | HP | Damage | XP | Gold | Respawn | Aggro Range |
|---------|-----|--------|-----|------|---------|-------------|
| Hollow Villager | 75 | 9 | 150 | 15-30 | 60s | 5 tiles |
| Corrupted Militia | 100 | 12 | 200 | 20-40 | 60s | 8 tiles |

**Location**: Around Fallen Outpost

### Boss - The First Fallen (Combat 30+)

| Stat | Value |
|------|-------|
| HP | 500 |
| Damage | 20 |
| XP | 400 |
| Gold | 75-150 |
| Respawn | 5 minutes |
| Aggro Range | 10 tiles |

**Location**: Corrupted Chapel interior

---

## Gear Progression & Stats

### Weapons

| Tier | Item | Atk Req | Attack | Strength | Source |
|------|------|---------|--------|----------|--------|
| 0 | Worn Pitchfork | 1 | +2 | +3 | Starting |
| 1 | Salvaged Sword | 1 | +6 | +8 | Quest 1 |
| 2 | Restored Blade | 10 | +12 | +15 | Quest 4 |
| 3 | Purified Longsword | 20 | +20 | +24 | Boss drop |
| 3 | Purified Longbow | 20 | +18 | +20 | Crafted (range 8) |

### Body Armor

| Tier | Item | Def Req | Defence | Source |
|------|------|---------|---------|--------|
| 0 | Torn Clothes | 1 | +1 | Starting |
| 1 | Salvaged Tunic | 1 | +5 | Pig/Slime drops, Shop |
| 2 | Restored Chainmail | 10 | +12 | Quest 6 |
| 3 | Purified Plate | 20 | +22 | Boss drop |

### Boots

| Tier | Item | Def Req | Defence | Source |
|------|------|---------|---------|--------|
| 0 | Worn Sandals | 1 | +0 | Starting |
| 1 | Salvaged Boots | 1 | +3 | Quest 3 |
| 2 | Restored Greaves | 10 | +7 | Quest 7 |
| 3 | Purified Sabatons | 20 | +12 | Boss drop |

### Helms

| Tier | Item | Def Req | Defence | Source |
|------|------|---------|---------|--------|
| 1 | Salvaged Hood | 1 | +2 | Spider/Wolf drops, Shop |
| 2 | Restored Helm | 10 | +6 | Villager/Militia drops |
| 3 | Purified Greathelm | 20 | +10 | Boss drop |

### Rings

| Tier | Item | Req | Atk | Str | Def | Source |
|------|------|-----|-----|-----|-----|--------|
| 1 | Salvaged Band | 1 | +2 | - | +2 | Quest 2 |
| 2 | Restored Signet | 10 | +4 | +4 | +4 | Quest 5 |
| 3 | Purified Loop | 20 | +6 | +6 | +8 | Boss drop |

### Gloves (Quest Capstone)

| Tier | Item | Req | Atk | Str | Def | Source |
|------|------|-----|-----|-----|-----|--------|
| 3 | Purified Gauntlets | 20 | +8 | +10 | +8 | Quest 8 |

---

## Drop Tables

### Corrupted Pig

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Spoiled Meat | 80% | 1 | Quest item |
| Tainted Hide | 30% | 1-2 | Crafting material |
| Salvaged Tunic | 5% | 1 | Tier 1 body |

### Blighted Slime

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Slime Core | 75% | 1-2 | Quest item, crafting |
| Corruption Essence | 25% | 1 | Crafting material |
| Salvaged Band | 3% | 1 | Tier 1 ring |

### Shadow Spider

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Tainted Webbing | 70% | 1-2 | Quest item |
| Spider Fang | 20% | 1 | Crafting material |
| Salvaged Hood | 8% | 1 | Tier 1 helm |

### Rotting Wolf

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Cursed Fang | 60% | 1 | Quest item |
| Matted Fur | 25% | 1-2 | Crafting material |
| Salvaged Hood | 8% | 1 | Tier 1 helm |

### Hollow Villager

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Tattered Cloth | 65% | 1-2 | Quest item |
| Faded Memory | 15% | 1 | Crafting/lore item |
| Restored Helm | 5% | 1 | Tier 2 helm |

### Corrupted Militia

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Militia Badge | 50% | 1 | Quest item |
| Bent Sword | 20% | 1 | Crafting material |
| Restored Helm | 8% | 1 | Tier 2 helm |

### The First Fallen (Boss)

| Drop | Rate | Qty | Notes |
|------|------|-----|-------|
| Corruption Shard | 100% | 2-5 | Guaranteed, crafting/vendor |
| Corrupted Bowstring | 25% | 1 | Bow part |
| Twisted Limbs | 20% | 1 | Bow part |
| Shadow Grip | 15% | 1 | Bow part |
| Purified Sabatons | 12% | 1 | Tier 3 boots |
| Purified Greathelm | 12% | 1 | Tier 3 helm |
| Purified Plate | 10% | 1 | Tier 3 body |
| Purified Loop | 8% | 1 | Tier 3 ring |
| Purified Longsword | 6% | 1 | Tier 3 weapon |

---

## Crafting Recipes

### Consumables

| Recipe | Level | Ingredients | Output |
|--------|-------|-------------|--------|
| Minor Health Potion | 1 | 3x Slime Core | 2x Health Potion |
| Health Potion | 3 | 5x Slime Core, 2x Corruption Essence | 3x Health Potion |
| Antidote | 2 | 2x Spider Fang, 2x Slime Core | 1x Antidote (cures poison) |

### Tier 1 Gear (Alternative to drops)

| Recipe | Level | Ingredients | Output |
|--------|-------|-------------|--------|
| Salvaged Tunic | 1 | 8x Tainted Hide | Salvaged Tunic |
| Salvaged Hood | 2 | 5x Tainted Hide, 3x Tainted Webbing | Salvaged Hood |
| Salvaged Boots | 2 | 6x Tainted Hide, 4x Matted Fur | Salvaged Boots |

### Tier 2 Gear

| Recipe | Level | Ingredients | Output |
|--------|-------|-------------|--------|
| Restored Helm | 8 | 10x Tattered Cloth, 5x Bent Sword | Restored Helm |
| Wolf Cloak | 5 | 12x Matted Fur, 5x Cursed Fang | Wolf Cloak (+4 Def, back slot) |

### Special - Purified Longbow

| Recipe | Level | Ingredients | Output |
|--------|-------|-------------|--------|
| Purified Longbow | 15 | 1x Corrupted Bowstring, 1x Twisted Limbs, 1x Shadow Grip | Purified Longbow |

### Utility

| Recipe | Level | Ingredients | Output |
|--------|-------|-------------|--------|
| Corruption Bundle | 1 | 10x Corruption Shard | 50 gold (vendor item) |
| Memory Fragment | 5 | 5x Faded Memory | Lore item (reveals backstory) |

---

## Shop Inventory

### Village Blacksmith (Ruined Village)

Located near Elder Mara. Sells Tier 0-1 gear as fallback.

| Item | Price | Stock | Restock |
|------|-------|-------|---------|
| Worn Pitchfork | 10g | 5 | 5 min |
| Torn Clothes | 15g | 5 | 5 min |
| Worn Sandals | 10g | 5 | 5 min |
| Salvaged Sword | 75g | 2 | 10 min |
| Salvaged Tunic | 60g | 3 | 10 min |
| Salvaged Boots | 50g | 3 | 10 min |
| Salvaged Hood | 40g | 3 | 10 min |

**Buy multiplier**: 0.5x (pays half value for player items)

### Wandering Alchemist (Forest Edge Camp)

Found near Scout Alara. Sells potions and buys materials.

| Item | Price | Stock | Restock |
|------|-------|-------|---------|
| Health Potion | 25g | 10 | 3 min |
| Antidote | 40g | 5 | 5 min |
| Slime Core | 8g | 20 | 5 min |
| Corruption Essence | 15g | 10 | 5 min |

**Buy multiplier**: 0.6x (slightly better rates for materials)

### Militia Quartermaster (Fallen Outpost)

Ghost/survivor who maintains the old supply cache.

| Item | Price | Stock | Restock |
|------|-------|-------|---------|
| Health Potion | 25g | 15 | 3 min |
| Salvaged Sword | 75g | 1 | 15 min |
| Salvaged Hood | 40g | 2 | 10 min |
| Arrows (if bow equipped) | 5g/20 | 100 | 1 min |

---

## Complete Material & Item List

### Materials (16 total)

| Material | Source | Primary Use |
|----------|--------|-------------|
| Spoiled Meat | Corrupted Pig | Quest 1 |
| Tainted Hide | Corrupted Pig | Crafting Tier 1 gear |
| Slime Core | Blighted Slime | Quest 2, potions |
| Corruption Essence | Blighted Slime | Potions |
| Tainted Webbing | Shadow Spider | Quest 4, crafting |
| Spider Fang | Shadow Spider | Antidote crafting |
| Cursed Fang | Rotting Wolf | Quest 5, crafting |
| Matted Fur | Rotting Wolf | Crafting |
| Tattered Cloth | Hollow Villager | Quest 6, crafting |
| Faded Memory | Hollow Villager | Lore item |
| Militia Badge | Corrupted Militia | Quest 7 |
| Bent Sword | Corrupted Militia | Crafting |
| Corruption Shard | The First Fallen | Vendor item |
| Corrupted Bowstring | The First Fallen | Bow crafting |
| Twisted Limbs | The First Fallen | Bow crafting |
| Shadow Grip | The First Fallen | Bow crafting |

### New NPCs (8 total)

| NPC | Location | Role |
|-----|----------|------|
| Elder Mara | Ruined Village | Quest giver |
| Farmer Jorik | Abandoned Farm | Quest giver |
| Scout Alara | Forest Edge Camp | Quest giver |
| Captain Roderick | Fallen Outpost | Quest giver |
| Spirit of Father Aldric | Corrupted Chapel | Quest giver |
| Blacksmith | Ruined Village | Merchant |
| Wandering Alchemist | Forest Edge Camp | Merchant |
| Militia Quartermaster | Fallen Outpost | Merchant |

### New Shops (3 total)

| Shop | Location | NPC |
|------|----------|-----|
| Village Blacksmith | Ruined Village | Blacksmith |
| Wandering Alchemist | Forest Edge Camp | Wandering Alchemist |
| Militia Quartermaster | Fallen Outpost | Militia Quartermaster |

---

## Level Progression Targets

| After Quest | Combat Level | Hitpoints |
|-------------|--------------|-----------|
| Quest 1 | 5-6 | 12-13 |
| Quest 2 | 8-9 | 15-16 |
| Quest 3 | 11-12 | 18-19 |
| Quest 4 | 15-16 | 22-23 |
| Quest 5 | 19-20 | 25-26 |
| Quest 6 | 24-25 | 29-30 |
| Quest 7 | 29-30 | 33-34 |
| Quest 8 + Boss Farm | 32-35 | 36-38 |

---

## Starting Equipment

Players begin with:

| Slot | Item | Stats |
|------|------|-------|
| Weapon | Worn Pitchfork | +2 Atk, +3 Str |
| Body | Torn Clothes | +1 Def |
| Feet | Worn Sandals | +0 Def |
| Gold | 20-30g | - |

No helm, ring, or gloves at start.
