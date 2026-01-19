# Skills Panel UI Design

## Overview

A compact, quick-reference skills panel showing combat skill levels at a glance with room for future expansion.

## Layout

**Panel Structure:**
- Position: Right side of screen, above bottom bar (same area as inventory/crafting)
- Size: ~180x200 pixels
- Header: "Skills" with combat level display (e.g., "Skills (Combat: 23)")
- Toggle: Opens/closes via Skills menu button or Escape key

**Grid:**
- 2 columns × 4 rows = 8 cells
- Cell size: ~40x40 pixels with 4px spacing
- Top 4 cells: Active skills
- Bottom 4 cells: Locked placeholders

```
[Hitpoints] [Attack  ]
[Strength ] [Defence ]
[Locked   ] [Locked  ]
[Locked   ] [Locked  ]
```

## Cell Design

**Active Skill Cell:**
- Dark slot background (matches inventory slots)
- Skill icon centered (16x16 or similar)
- Level number in bottom-right corner (white text, dark shadow)
- Hover state: lighter background, triggers tooltip

**Locked Cell:**
- Darker/dimmed background
- Lock icon centered (padlock shape)
- No hover interaction, no tooltip

## Tooltip

Shown when hovering an active skill cell:
- Skill name in gold/title color
- "Level: 45"
- "XP: 61,512 / 67,983"
- "To next level: 6,471 XP"

Uses existing tooltip styling (dark background, medieval border).

## Styling

- Panel frame matches existing UI (inventory, crafting, quest log)
- Colors from common.rs palette (SLOT_BG_EMPTY, SLOT_HOVER_BG, etc.)
- Text uses existing bitmap font

## Not Included (YAGNI)

- XP progress bars in cells
- Click interactions on skills
- Skill icons for locked slots
- Detailed stat breakdowns

## Implementation Notes

**New files:**
- `client/src/render/ui/skills.rs` - Panel rendering

**Modified files:**
- `client/src/render/ui/mod.rs` - Export skills module
- `client/src/render/renderer.rs` - Call render_skills_panel
- `client/src/ui/layout.rs` - Add UiElementId::SkillSlot(usize)

**Assets needed:**
- Skill icons (16x16): hitpoints, attack, strength, defence
- Lock icon (16x16 or drawn with primitives)
