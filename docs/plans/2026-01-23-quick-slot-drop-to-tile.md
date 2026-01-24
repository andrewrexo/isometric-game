# Quick Slot Drop-to-Tile Feature

## Overview

Allow players to drag items from quick slots and drop them onto adjacent tiles (1-tile radius around character).

## Drop Target Detection

Valid drop targets are the 8 tiles surrounding the player (cardinal + diagonal):

```
[NW] [N] [NE]
[W]  [P] [E]
[SW] [S] [SE]
```

Detection logic:
1. Convert mouse screen position to world tile coordinates
2. Calculate Chebyshev distance from player's current tile
3. Valid if distance = 1 (not player's own tile)
4. Highlight valid tiles when dragging

## Drop Mechanics

- Normal drag release: drop entire stack at target tile
- Ctrl/Cmd held during release: drop single item, rest stays in slot
- Reuse `InputCommand::DropItem` with added target tile coordinates
- Server validates tile is adjacent and walkable
- Invalid drops keep item in inventory (no optimistic removal)

## Clean Drag Visual

- Render only the item sprite centered on cursor
- No slot background, borders, padding, or quantity badge
- Semi-transparent (70% opacity)
- Valid tile: subtle green highlight
- Invalid area: no highlight

## Files to Modify

1. `client/src/input/handler.rs` - Drop-to-tile logic, Ctrl/Cmd detection, adjacency validation
2. `client/src/render/ui/quick_slots.rs` - Clean drag ghost at cursor
3. `client/src/game/state.rs` - Track target tile during drag for feedback
4. Server - Update DropItem command with optional target coordinates
