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
pub const HEAD_HAIR_FRAGMENT: &str = r#"#version 100
precision lowp float;

varying vec2 uv;

uniform sampler2D Texture;      // Head sprite (primary texture)
uniform sampler2D HairTexture;  // Hair sprite

void main() {
    vec4 head = texture2D(Texture, uv);
    vec4 hair = texture2D(HairTexture, uv);

    // Detect (8,0,0) "show hair" marker
    // 8/255 = 0.0314, check with tolerance
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
"#;
