# Map Editor Portal Tool - Design

## Overview

Add a portal placement tool to the map editor so portals can be visually placed and configured, rather than manually editing JSON files.

## Data Structure

**Portal interface** (add to types):
```typescript
interface Portal {
  id: string;           // Unique ID (auto-generated)
  x: number;            // Local tile X within chunk
  y: number;            // Local tile Y within chunk
  width: number;        // Width in tiles (default 1)
  height: number;       // Height in tiles (default 1)
  targetMap: string;    // Interior map ID (e.g., "test_house")
  targetSpawn: string;  // Spawn point name (e.g., "entrance")
}
```

**Chunk extension:**
```typescript
interface Chunk {
  // ... existing fields
  portals: Portal[];
}
```

## Tool Behavior

- **Keyboard shortcut:** P
- **Click empty tile:** Creates new 1x1 portal with empty target
- **Click existing portal:** Selects it for editing
- **Visualization:** Semi-transparent purple rectangle over portal tiles

## Properties Panel

When a portal is selected, show:
- **Position:** X, Y (read-only world coordinates)
- **Size:** Width, Height inputs (min 1)
- **Target Map:** Text input for interior map ID
- **Target Spawn:** Text input for spawn point name
- **Delete button**

## Layer Visibility

- Portals render on their own visual layer
- "Show Portals" checkbox in layer panel

## Files to Modify

1. **`/mapper/src/types/index.ts`**
   - Add Portal interface
   - Add `portals: Portal[]` to Chunk interface
   - Add `Portal = 'portal'` to Tool enum

2. **`/mapper/src/state/store.ts`**
   - Add `selectedPortal: { chunkCoord: ChunkCoord, portalId: string } | null`
   - Add `showPortals: boolean` (default true)
   - Add `addPortal(worldTile: Point): void`
   - Add `updatePortal(chunkCoord: ChunkCoord, portalId: string, updates: Partial<Portal>): void`
   - Add `removePortal(chunkCoord: ChunkCoord, portalId: string): void`
   - Add `setSelectedPortal(selection: { chunkCoord, portalId } | null): void`
   - Initialize `portals: []` when creating new chunks

3. **`/mapper/src/components/Toolbar/index.tsx`**
   - Add Portal tool button: `{ id: Tool.Portal, label: 'Portal', shortcut: 'P' }`

4. **`/mapper/src/components/Canvas/index.tsx`**
   - Add Portal tool case in handleToolAction click handler
   - Add portal rendering (purple semi-transparent rectangles)
   - Add findPortalAtWorld helper function

5. **`/mapper/src/components/PropertiesPanel/index.tsx`**
   - Add portal properties section when selectedPortal is set
   - Width, Height number inputs
   - Target Map, Target Spawn text inputs
   - Delete button

6. **`/mapper/src/components/LayerPanel/index.tsx`**
   - Add "Show Portals" checkbox toggle

7. **`/mapper/src/core/Storage.ts`**
   - Ensure portals array serialized with chunks (should auto-work if in Chunk type)
   - Handle missing portals field in old chunks (default to empty array)

## Serialization

Portals serialize to chunk JSON matching server format:
```json
{
  "portals": [
    {
      "id": "portal_abc123",
      "x": 16,
      "y": 16,
      "width": 1,
      "height": 1,
      "targetMap": "test_house",
      "targetSpawn": "entrance"
    }
  ]
}
```

## Visual Style

- **Color:** Purple with 50% opacity (`rgba(128, 0, 255, 0.5)`)
- **Selected:** Brighter purple border or highlight
- **Size:** Covers full tile(s) based on width/height
