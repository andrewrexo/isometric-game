# Head Slot Rendering Design

## Overview

Add head equipment rendering with a custom shader that composites hair and head sprites. The shader uses special pixel colors in head sprites to control where player hair is visible, allowing for helmets that fully hide hair, hoods that show some hair, etc.

## Sprite Format

### Head Sprite Layout

- 2-frame horizontal strip: `[front, back]`
- Frame 0: Front-facing (Down, Right directions)
- Frame 1: Back-facing (Up, Left directions)
- Dimensions: 34px width (same as player), height varies by item

### Hair Masking Convention

Head sprites use special pixel values to control hair visibility:

| Pixel Value | Meaning |
|-------------|---------|
| Fully transparent (alpha = 0) | Hair is cut off / hidden here |
| RGB (8, 0, 0) with alpha > 0 | Hair shows through here |
| Any other color | Head sprite is drawn (covers hair) |

This allows artists to create:
- Full helmets: No (8,0,0) pixels, hair completely hidden
- Hoods: (8,0,0) where hair should peek out
- Hats: Mix of coverage and hair visibility

## Shader Design

### Fragment Shader Logic

```glsl
#version 100
precision lowp float;

varying vec2 uv;
uniform sampler2D Texture;      // Head sprite (primary)
uniform sampler2D HairTexture;  // Hair sprite

void main() {
    vec4 head = texture2D(Texture, uv);
    vec4 hair = texture2D(HairTexture, uv);

    // Detect (8,0,0) "show hair" marker
    // 8/255 ≈ 0.031, check with small tolerance
    bool is_hair_marker = head.r > 0.02 && head.r < 0.05
                       && head.g < 0.01
                       && head.b < 0.01
                       && head.a > 0.5;

    if (is_hair_marker) {
        gl_FragColor = hair;           // Show hair through
    } else if (head.a > 0.01) {
        gl_FragColor = head;           // Show head
    } else {
        gl_FragColor = vec4(0.0);      // Transparent = hair cut off
    }
}
```

### Vertex Shader

Standard passthrough vertex shader - transforms position and passes UV coordinates.

## Animation & Offsets

### Frame Selection

```rust
fn get_head_frame(direction: Direction) -> FrameInfo {
    let is_back = matches!(direction, Direction::Up | Direction::Left);
    let flip_h = matches!(direction, Direction::Up | Direction::Right);
    FrameInfo {
        frame: if is_back { 1 } else { 0 },
        flip_h,
    }
}
```

### Offset Function

```rust
fn get_head_offset(state: AnimationState, direction: Direction, frame: u32) -> (f32, f32) {
    // Base offset: aligned to top of player sprite
    // Attack frame 2 and ShootingBow get adjusted offsets (same as hair)
    // Start with (0.0, 0.0), tune based on visual testing
}
```

Offsets will mirror hair offsets since head and hair move together with the player's head.

## Render Order

Updated `render_player()` sequence:

1. Shadow
2. Weapon under-layer
3. Player base sprite
4. **Head+Hair composite** (when head equipped) OR **Hair only** (no head equipped)
5. Body armor
6. Boots
7. Weapon over-layer
8. Name & health bar

Key change: Hair rendering becomes conditional - either rendered via shader composite with head, or rendered standalone as it currently works.

## Implementation

### New Files

| File | Purpose |
|------|---------|
| `client/src/render/shaders.rs` | GLSL shader source code for head+hair composite |

### Modified Files

| File | Changes |
|------|---------|
| `client/src/render/renderer.rs` | Add `head_hair_material: Material`, load in `new()`, render head in `render_player()` |
| `client/src/render/animation.rs` | Add `HEAD_SPRITE_WIDTH/HEIGHT`, `get_head_frame()`, `get_head_offset()` |
| `client/src/render/mod.rs` | Add `mod shaders;` |

### Renderer Changes

```rust
// In Renderer struct
head_hair_material: Material,

// In Renderer::new()
let head_hair_material = load_material(
    ShaderSource::Glsl {
        vertex: shaders::HEAD_HAIR_VERTEX,
        fragment: shaders::HEAD_HAIR_FRAGMENT,
    },
    MaterialParams {
        textures: vec!["HairTexture".to_string()],
        ..Default::default()
    },
).unwrap();

// In render_player()
if let Some(ref head_item_id) = player.equipped_head {
    if let Some(head_tex) = self.equipment_sprites.get(head_item_id) {
        if let Some(hair_tex) = self.hair_sprites.get(&player.hair_style.unwrap_or(0)) {
            // Use shader composite
            gl_use_material(&self.head_hair_material);
            self.head_hair_material.set_texture("HairTexture", *hair_tex);
            // Draw head sprite (shader composites hair automatically)
            draw_texture_ex(head_tex, ...);
            gl_use_default_material();
        }
    }
} else {
    // Render hair normally (existing code)
}
```

## Assets

Head sprites already added in `client/assets/sprites/equipment/head/`:
- `wizard_hat.png`
- `hood_black.png`
- `hood_brown.png`
- `helmet_of_darkness.png`

## Future Considerations

- **More head frames**: Could expand to 22-frame format like body armor if head bobbing is desired
- **Hair color tinting**: Shader could apply hair color tint uniform instead of using pre-colored hair sprites
- **Partial transparency**: Shader could blend hair and head for semi-transparent visors
