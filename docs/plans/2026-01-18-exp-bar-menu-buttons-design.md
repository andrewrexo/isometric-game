# Experience Bar & Menu Buttons Design

## Overview

Add two UI elements to the game:
1. A full-width experience bar at the bottom edge of the screen
2. A row of menu buttons in the bottom-right corner

## Experience Bar

**Layout:**
- Full-width at the very bottom edge of the screen
- Height: ~20px
- 8px padding on left/right edges
- 10px gap between exp bar and UI elements above it

**Visuals:**
- Background: `PANEL_BG_DARK`
- Fill: gold/amber accent color showing progress
- Border: bronze frame matching existing UI theme
- Text: centered, "EXP: {current} / {total}"

**Data source:**
- `player.exp` and `player.exp_to_next_level` from Player struct

## Menu Buttons

**Layout:**
- 5 buttons in a horizontal row
- Position: bottom-right corner, above the exp bar
- Button size: 40x40px
- Spacing: 4px between buttons

**Button order (left to right):**
1. Character/Stats
2. Inventory
3. Social
4. Map
5. Settings

**Visuals:**
- Bronze-framed dark panel style (matches hotbar slots)
- Hover state: highlight effect
- Active state: brighter border when panel is open
- Placeholder icons until real icons are added

**Functionality:**
| Button | Action |
|--------|--------|
| Character | Toggle `character_open` (new) |
| Inventory | Toggle `inventory_open` (existing) |
| Social | Toggle `social_open` (new) |
| Map | Toggle `map_open` (new) |
| Settings | Open escape menu (existing) |

## State & Behavior

**New UiState fields:**
- `character_open: bool`
- `social_open: bool`
- `map_open: bool`

**New UiElementId variants:**
- `MenuButtonCharacter`
- `MenuButtonInventory`
- `MenuButtonSocial`
- `MenuButtonMap`
- `MenuButtonSettings`

**Mutual exclusivity:**
- Panels (Inventory, Character, Social, Map) are mutually exclusive
- Opening one auto-closes any other open panel
- Settings/escape menu is independent

## Files to Modify

1. `client/src/game/state.rs` - add new UiState fields
2. `client/src/ui/layout.rs` - add new UiElementId variants
3. `client/src/render/ui/common.rs` - add constants for button size, exp bar height
4. `client/src/render/renderer.rs` - render exp bar and menu buttons
5. `client/src/input/handler.rs` - handle button click events
