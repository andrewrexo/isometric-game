//! Custom shaders for equipment rendering

/// Vertex shader for head+hair composite rendering
pub const HEAD_HAIR_VERTEX: &str = r#"#version 100
precision lowp float;

attribute vec3 position;
attribute vec2 texcoord;

varying vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}
"#;

/// Fragment shader for head+hair composite rendering
///
/// This shader composites hair and head sprites based on special pixel values:
/// - Fully transparent pixels in head: hair is cut off (not visible)
/// - RGB (8, 0, 0) pixels in head: hair shows through
/// - Any other pixels: head is drawn (covers hair)
///
/// Uniforms:
/// - Texture: Head sprite sheet (primary)
/// - HairTexture: Hair sprite sheet
/// - HairUvTransform: vec4(offset_x, offset_y, scale_x, scale_y) to transform head UV to hair UV
/// - Tint: vec4 color tint to apply (default white with full alpha for normal rendering)
pub const HEAD_HAIR_FRAGMENT: &str = r#"#version 100
precision lowp float;

varying vec2 uv;

uniform sampler2D Texture;      // Head sprite (with source rect already applied by macroquad)
uniform sampler2D HairTexture;  // Hair sprite (full texture, we compute UV)
uniform vec4 HairUvTransform;   // (offset_x, offset_y, scale_x, scale_y)
uniform vec4 Tint;              // Color tint (default: 1,1,1,1 for no tint)

void main() {
    vec4 head = texture2D(Texture, uv);

    // Transform UV from head space to hair space
    vec2 hair_uv = HairUvTransform.xy + uv * HairUvTransform.zw;

    // Check if hair UV is in valid range [0,1]
    bool hair_in_bounds = hair_uv.x >= 0.0 && hair_uv.x <= 1.0 && hair_uv.y >= 0.0 && hair_uv.y <= 1.0;

    vec4 hair = vec4(0.0);
    if (hair_in_bounds) {
        hair = texture2D(HairTexture, hair_uv);
    }

    // Detect (8,0,0) "show hair" marker
    // 8/255 = 0.0314, check with wider tolerance
    bool is_hair_marker = head.r > 0.01 && head.r < 0.08
                       && head.g < 0.02
                       && head.b < 0.02
                       && head.a > 0.5;

    if (is_hair_marker) {
        // Hair marker detected - show hair if in bounds, otherwise discard
        if (hair_in_bounds && hair.a > 0.01) {
            gl_FragColor = hair * Tint;
        } else {
            discard;  // No hair here, discard the marker pixel
        }
    } else if (head.a > 0.01) {
        gl_FragColor = head * Tint;    // Show head (opaque non-marker pixels)
    } else {
        discard;                       // Transparent = discard
    }
}
"#;
