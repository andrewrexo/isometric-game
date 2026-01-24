use crate::game::Direction;
pub const SPRITE_WIDTH: f32 = 34.0;
pub const SPRITE_HEIGHT: f32 = 78.0;

// Weapon sprite dimensions (single row, 17-18 frames per weapon)
pub const WEAPON_SPRITE_WIDTH: f32 = 68.0;
pub const WEAPON_SPRITE_HEIGHT: f32 = 84.0;

// Boot sprite dimensions (single row format, 16 frames per boot)
pub const BOOT_SPRITE_WIDTH: f32 = 34.0;
pub const BOOT_SPRITE_HEIGHT: f32 = 47.0;

// Body armor sprite dimensions (single row format, 22 frames per armor)
pub const BODY_ARMOR_SPRITE_WIDTH: f32 = 34.0;  // Same as player sprite width
pub const BODY_ARMOR_SPRITE_HEIGHT: f32 = 77.0;

// Head equipment sprite dimensions (single row format, 2 frames: front/back)
pub const HEAD_SPRITE_WIDTH: f32 = 30.0;
pub const HEAD_SPRITE_HEIGHT: f32 = 34.0;

/// Animation states the player can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Idle,
    Walking,
    Attacking,
    SittingGround,
    SittingChair,
    Casting,
    ShootingBow,
}

impl Default for AnimationState {
    fn default() -> Self {
        AnimationState::Idle
    }
}

/// Animation configuration for each state
#[derive(Debug, Clone, Copy)]
pub struct AnimationConfig {
    /// Starting row in the sprite sheet for this animation
    pub base_row: u32,
    /// Number of frames in this animation
    pub frame_count: u32,
    /// Frames per second for this animation
    pub fps: f32,
    /// Whether this animation should loop
    pub looping: bool,
    /// Whether this animation uses directional rows (adds direction offset to base_row)
    pub directional: bool,
}

impl AnimationConfig {
    const fn new(base_row: u32, frame_count: u32, fps: f32, looping: bool, directional: bool) -> Self {
        Self { base_row, frame_count, fps, looping, directional }
    }
}

/// Sprite sheet layout - mapping animation states to rows
/// Based on the actual sprite sheet (211x312, 6 cols x 4 rows):
/// Row 0: Idle (0-1), Walking down/right (2-5)
/// Row 1: Walking up/left (0-3), Casting (4-5)
/// Row 2: Attack down/right (0-1), Attack up/left (2-3), Sitting chair (4-5)
/// Row 3: Sitting ground (0-1), Shooting bow (2-3)
///
/// Flip sprite horizontally for up/right directions
pub fn get_animation_config(state: AnimationState) -> AnimationConfig {
    match state {
        // Idle uses row 0, frames 0-1, directional (will use frame offset)
        AnimationState::Idle => AnimationConfig::new(0, 1, 2.0, true, false),
        // Walking is directional - down/right on row 0, up/left on row 1
        AnimationState::Walking => AnimationConfig::new(0, 4, 10.0, true, true),
        // Attacking is directional - down/right frames 0-1, up/left frames 2-3 on row 2
        AnimationState::Attacking => AnimationConfig::new(2, 2, 7.0, false, true),
        // Sitting chair on row 2, frames 4-5
        AnimationState::SittingChair => AnimationConfig::new(2, 2, 1.0, true, false),
        // Sitting ground on row 3, frames 0-1
        AnimationState::SittingGround => AnimationConfig::new(3, 2, 1.0, true, false),
        // Casting on row 1, frames 4-5
        AnimationState::Casting => AnimationConfig::new(1, 2, 6.0, false, false),
        // Shooting bow on row 3, frame 2 for down/right, frame 3 for up/left (single frame per direction)
        AnimationState::ShootingBow => AnimationConfig::new(3, 1, 7.0, false, false),
    }
}

/// Check if direction uses up/left animations (row 1 for walking, frames 2-3 for attack)
pub fn is_up_or_left_direction(direction: Direction) -> bool {
    matches!(direction, Direction::Up | Direction::Left)
}

/// Whether to flip the sprite horizontally for this direction
/// Flip for Up and Right directions
pub fn should_flip_horizontal(direction: Direction) -> bool {
    matches!(direction, Direction::Up | Direction::Right)
}

/// Player animation controller
#[derive(Debug, Clone)]
pub struct PlayerAnimation {
    pub state: AnimationState,
    pub frame: f32,
    pub direction: Direction,
    /// Callback for when a non-looping animation completes
    finished: bool,
}

impl Default for PlayerAnimation {
    fn default() -> Self {
        Self {
            state: AnimationState::Idle,
            frame: 0.0,
            direction: Direction::Down,
            finished: false,
        }
    }
}

impl PlayerAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the animation frame based on delta time
    pub fn update(&mut self, delta: f32) {
        let config = get_animation_config(self.state);

        self.frame += delta * config.fps;

        if self.frame >= config.frame_count as f32 {
            if config.looping {
                self.frame = self.frame % config.frame_count as f32;
            } else {
                self.frame = (config.frame_count - 1) as f32;
                self.finished = true;
            }
        }
    }

    /// Set a new animation state, resetting the frame if changed
    pub fn set_state(&mut self, new_state: AnimationState) {
        if self.state != new_state {
            self.state = new_state;
            self.frame = 0.0;
            self.finished = false;
        }
    }

    /// Check if a non-looping animation has finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Get the current sprite sheet coordinates
    pub fn get_sprite_coords(&self) -> SpriteCoords {
        let config = get_animation_config(self.state);
        let frame_index = self.frame as u32 % config.frame_count;
        let use_up_left_anim = is_up_or_left_direction(self.direction);

        let (row, col) = match self.state {
            AnimationState::Idle => {
                // Row 0, frames 0-1
                // Frame 0 for down/right, frame 1 for up/left
                if use_up_left_anim {
                    (0, 1)
                } else {
                    (0, 0)
                }
            }
            AnimationState::Walking => {
                // Row 1 (frames 0-3) for up/left directions
                // Row 0 (frames 2-5) for down/right directions
                if use_up_left_anim {
                    (1, frame_index)
                } else {
                    (0, 2 + frame_index)
                }
            }
            AnimationState::Attacking => {
                // Row 2, frames 2-3 for up/left, frames 0-1 for down/right
                if use_up_left_anim {
                    (2, 2 + frame_index)
                } else {
                    (2, frame_index)
                }
            }
            AnimationState::SittingChair => {
                // Row 2, frames 4-5
                (2, 4 + frame_index)
            }
            AnimationState::SittingGround => {
                // Row 3, frames 0-1
                (3, frame_index)
            }
            AnimationState::Casting => {
                // Row 1, frames 4-5
                (1, 4 + frame_index)
            }
            AnimationState::ShootingBow => {
                // Row 3, frame 2 for down/right, frame 3 for up/left
                if use_up_left_anim {
                    (3, 3)
                } else {
                    (3, 2)
                }
            }
        };

        SpriteCoords {
            col,
            row,
            flip_h: should_flip_horizontal(self.direction),
        }
    }
}

/// Sprite coordinates in the sprite sheet
#[derive(Debug, Clone, Copy)]
pub struct SpriteCoords {
    pub col: u32,
    pub row: u32,
    pub flip_h: bool,
}

impl SpriteCoords {
    /// Get the source rectangle in the sprite sheet (in pixels)
    pub fn to_source_rect(&self) -> (f32, f32, f32, f32) {
        let x = self.col as f32 * SPRITE_WIDTH;
        let y = self.row as f32 * SPRITE_HEIGHT;
        (x, y, SPRITE_WIDTH, SPRITE_HEIGHT)
    }
}

// ============================================================================
// NPC Animation System
// ============================================================================

/// Animation states for NPCs (simpler than players)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpcAnimationState {
    #[default]
    Idle,
    Walking,
    Attacking,
}

/// NPC animation controller
///
/// NPC sprites use a single-row 16-frame layout:
/// - Frames 0-1: Idle (Down/Right)
/// - Frames 2-3: Idle (Up/Left)
/// - Frames 4-7: Walk (Down/Right)
/// - Frames 8-11: Walk (Up/Left)
/// - Frames 12-13: Attack (Down/Right)
/// - Frames 14-15: Attack (Up/Left)
#[derive(Debug, Clone)]
pub struct NpcAnimation {
    pub state: NpcAnimationState,
    pub frame: f32,
    finished: bool,
}

impl Default for NpcAnimation {
    fn default() -> Self {
        Self {
            state: NpcAnimationState::Idle,
            frame: 0.0,
            finished: false,
        }
    }
}

impl NpcAnimation {
    /// Animation speeds for each state (matched to player speeds)
    const IDLE_FPS: f32 = 2.0;
    const WALK_FPS: f32 = 10.0;
    const ATTACK_FPS: f32 = 6.0;

    /// Frame counts for each state
    const IDLE_FRAMES: u32 = 2;
    const WALK_FRAMES: u32 = 4;
    const ATTACK_FRAMES: u32 = 2;

    /// Update the animation frame based on delta time
    pub fn update(&mut self, delta: f32) {
        let (frame_count, fps, looping) = match self.state {
            NpcAnimationState::Idle => (Self::IDLE_FRAMES, Self::IDLE_FPS, true),
            NpcAnimationState::Walking => (Self::WALK_FRAMES, Self::WALK_FPS, true),
            NpcAnimationState::Attacking => (Self::ATTACK_FRAMES, Self::ATTACK_FPS, false),
        };

        self.frame += delta * fps;

        if self.frame >= frame_count as f32 {
            if looping {
                self.frame = self.frame % frame_count as f32;
            } else {
                self.frame = (frame_count - 1) as f32;
                self.finished = true;
            }
        }
    }

    /// Set a new animation state, resetting the frame if changed
    pub fn set_state(&mut self, new_state: NpcAnimationState) {
        if self.state != new_state {
            self.state = new_state;
            self.frame = 0.0;
            self.finished = false;
        }
    }

    /// Check if a non-looping animation has finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Restart the current animation from the beginning
    pub fn restart(&mut self) {
        self.frame = 0.0;
        self.finished = false;
    }

    /// Get the sprite frame index (0-15) based on state and direction
    pub fn get_frame_index(&self, direction: Direction) -> u32 {
        let use_up_left = is_up_or_left_direction(direction);
        let frame_in_anim = self.frame as u32;

        match self.state {
            NpcAnimationState::Idle => {
                // Use only first idle frame (not all enemies have 2 idle frames)
                if use_up_left { 2 } else { 0 }
            }
            NpcAnimationState::Walking => {
                let base = if use_up_left { 8 } else { 4 };
                base + (frame_in_anim % Self::WALK_FRAMES)
            }
            NpcAnimationState::Attacking => {
                let base = if use_up_left { 14 } else { 12 };
                base + (frame_in_anim % Self::ATTACK_FRAMES)
            }
        }
    }

    /// Whether to flip the sprite horizontally for this direction
    /// Matches player flip logic: flip for Up, UpRight, Right, UpLeft
    pub fn should_flip(direction: Direction) -> bool {
        should_flip_horizontal(direction)
    }
}

// ============================================================================
// Weapon Animation System
// ============================================================================

/// Result of weapon frame calculation
/// Contains frame indices and whether to flip horizontally
#[derive(Debug, Clone, Copy)]
pub struct WeaponFrameResult {
    /// Frame to render under player (always present)
    pub frame_under: u32,
    /// Optional frame to render over player (for attack frame 2 front directions)
    pub frame_over: Option<u32>,
    /// Whether to flip the sprite horizontally
    pub flip_h: bool,
}

/// Get the weapon frame indices for the current animation state and direction
///
/// Weapon sprite sheet layout (0-indexed):
/// - 0: Standing front (Down/Right)
/// - 1: Standing back (Up/Left)
/// - 2-5: Walking front (Down/Right)
/// - 6-9: Walking back (Up/Left)
/// - 10: SpellCast front
/// - 11: SpellCast back
/// - 12: Attack frame 1 front
/// - 13: Attack frame 2 front (rendered UNDER player)
/// - 14: Attack frame 1 back
/// - 15: Attack frame 2 back
/// - 16: Attack frame 2 front overlay (rendered OVER player - hilt erased)
/// - 17: ShootingBow (ranged weapons only)
pub fn get_weapon_frame(state: AnimationState, direction: Direction, anim_frame: u32) -> WeaponFrameResult {
    let use_back = is_up_or_left_direction(direction);
    let flip_h = should_flip_horizontal(direction);

    let (frame_under, frame_over) = match state {
        AnimationState::Idle => {
            if use_back { (1, None) } else { (0, None) }
        }
        AnimationState::Walking => {
            let frame_in_walk = anim_frame % 4;
            if use_back {
                (6 + frame_in_walk, None)
            } else {
                (2 + frame_in_walk, None)
            }
        }
        AnimationState::Casting => {
            if use_back { (11, None) } else { (10, None) }
        }
        AnimationState::Attacking => {
            let attack_frame = anim_frame % 2;
            if use_back {
                // Back attack: frames 14-15, no overlay
                if attack_frame == 0 { (14, None) } else { (15, None) }
            } else {
                // Front attack: frame 0 uses 12, frame 1 uses 13 under + 16 over
                if attack_frame == 0 {
                    (12, None)
                } else {
                    (13, Some(16))
                }
            }
        }
        AnimationState::ShootingBow => {
            // Use frame 17 for all directions (flipped horizontally for up/left)
            (17, None)
        }
        // Sitting animations don't show weapons
        AnimationState::SittingGround | AnimationState::SittingChair => {
            if use_back { (1, None) } else { (0, None) }
        }
    };

    WeaponFrameResult {
        frame_under,
        frame_over,
        flip_h,
    }
}

/// Get the pixel offset for weapon positioning relative to the player sprite
///
/// Returns (x_offset, y_offset) to adjust weapon position based on animation state and direction.
/// These offsets account for the difference in sprite sizes (weapon: 68x84, player: 34x78)
/// and align the weapon with the player's hand position.
///
/// Initial values are conservative defaults - tune visually during testing.
pub fn get_weapon_offset(state: AnimationState, direction: Direction, anim_frame: u32) -> (f32, f32) {
    let use_back = is_up_or_left_direction(direction);

    // Base offset to center the larger weapon sprite over the player
    // Weapon is 68 wide, player is 34, so weapon needs to shift left by (68-34)/2 = 17
    // Weapon is 84 tall, player is 78, so weapon needs to shift up by (84-78)/2 = 3
    let base_x = -17.0;
    let base_y = -3.0;

    // Additional per-state/frame offsets for hand alignment
    let (state_x, state_y) = match state {
        AnimationState::Idle => {
            // Raise weapon to align with hand
            if use_back { (-8.0, -8.0) } else { (-7.0, -6.0) }
        }
        AnimationState::Walking => {
            // Raise weapon + slight bounce during walk cycle
            let walk_frame = anim_frame % 4;
            if use_back {
                match walk_frame {
                    0 => (-8.0, -8.0),
                    1 => (-8.0, -9.0),
                    2 => (-8.0, -8.0),
                    3 => (-8.0, -9.0),
                    _ => (-8.0, -8.0),
                }
            } else {
                match walk_frame {
                    0 => (-6.0, -8.0),
                    1 => (-6.0, -9.0),
                    2 => (-6.0, -8.0),
                    3 => (-6.0, -9.0),
                    _ => (-6.0, -8.0),
                }
            }
        }
        AnimationState::Attacking => {
            let attack_frame = anim_frame % 2;
            if use_back {
                // Shift for back-facing attacks
                if attack_frame == 0 { (-7.0, -6.0) } else { (-7.0, -6.0) }
            } else {
                // Shift left for front-facing attacks
                if attack_frame == 0 { (-5.0, 0.0) } else { (-5.0, 0.0) }
            }
        }
        AnimationState::Casting => (0.0, 0.0),
        AnimationState::ShootingBow => (-12.0, -4.0), // Shift bow left and up (mirrored for right/up)
        AnimationState::SittingGround | AnimationState::SittingChair => (0.0, 0.0),
    };

    // Invert state x offset when sprite is flipped horizontally to maintain hand alignment
    // (base_x is for centering and stays the same, state_x is for hand position and flips)
    let adjusted_state_x = if should_flip_horizontal(direction) {
        -state_x
    } else {
        state_x
    };

    (base_x + adjusted_state_x, base_y + state_y)
}

// ============================================================================
// Boot Animation System (single-row sprite format)
// ============================================================================

/// Result of boot frame calculation
#[derive(Debug, Clone, Copy)]
pub struct BootFrameResult {
    /// Frame index (0-based) in the single-row spritesheet
    pub frame: u32,
    /// Whether to flip the sprite horizontally
    pub flip_h: bool,
}

/// Get the boot frame index for the current animation state and direction
///
/// Boot sprite sheet layout (0-indexed frames):
/// - 0: Standing front (Down/Right)
/// - 1: Standing back (Up/Left)
/// - 2-5: Walking front (Down/Right)
/// - 6-9: Walking back (Up/Left)
/// - 10: Attack front
/// - 11: Attack back
/// - 12: Sit chair front
/// - 13: Sit chair back
/// - 14: Sit ground front
/// - 15: Sit ground back
pub fn get_boot_frame(state: AnimationState, direction: Direction, anim_frame: u32) -> BootFrameResult {
    let use_back = is_up_or_left_direction(direction);
    let flip_h = should_flip_horizontal(direction);

    let frame = match state {
        AnimationState::Idle => {
            if use_back { 1 } else { 0 }
        }
        AnimationState::Walking => {
            let frame_in_walk = anim_frame % 4;
            if use_back {
                6 + frame_in_walk // Frames 6-9
            } else {
                2 + frame_in_walk // Frames 2-5
            }
        }
        AnimationState::Attacking => {
            // Only use attack frame for 2nd attack frame, use idle for 1st
            let attack_frame = anim_frame % 2;
            if attack_frame == 0 {
                // 1st attack frame: use idle
                if use_back { 1 } else { 0 }
            } else {
                // 2nd attack frame: use attack
                if use_back { 11 } else { 10 }
            }
        }
        AnimationState::Casting => {
            // Use standing frame for casting
            if use_back { 1 } else { 0 }
        }
        AnimationState::ShootingBow => {
            // Use attack frames for shooting
            if use_back { 11 } else { 10 }
        }
        AnimationState::SittingChair => {
            if use_back { 13 } else { 12 }
        }
        AnimationState::SittingGround => {
            if use_back { 15 } else { 14 }
        }
    };

    BootFrameResult { frame, flip_h }
}

/// Get the pixel offset for boot positioning relative to the player sprite
///
/// Boots are positioned at the player's feet. These offsets adjust the boot sprite
/// to align properly with the player's foot position in each animation frame.
pub fn get_boot_offset(state: AnimationState, direction: Direction, anim_frame: u32) -> (f32, f32) {
    let use_back = is_up_or_left_direction(direction);

    // Base offset: boots are 34 wide (same as player), so center them
    // Boots are 47 tall and should align with player's feet
    let base_x = 0.0; // Center boots under player
    let base_y = 46.0; // Position at feet

    // Per-state offsets for alignment
    let (state_x, state_y) = match state {
        AnimationState::Idle => {
            // Up/Left: down 1, left 1 for left (mirrored for up)
            // Down/Right: no offset
            if use_back { (-1.0, 1.0) } else { (0.0, 0.0) }
        }
        AnimationState::Walking => {
            let walk_frame = anim_frame % 4;
            // Down/Right: right 1, down 2 (mirrored for right)
            // Up/Left: slight bounce only
            if use_back {
                match walk_frame {
                    0 => (0.0, 0.0),
                    1 => (0.0, -1.0),
                    2 => (0.0, 0.0),
                    3 => (0.0, -1.0),
                    _ => (0.0, 0.0),
                }
            } else {
                // Down/Right (mirrored for right)
                match walk_frame {
                    0 => (0.0, 1.0),
                    1 => (0.0, 0.0),
                    2 => (0.0, 1.0),
                    3 => (0.0, 0.0),
                    _ => (0.0, 1.0),
                }
            }
        }
        AnimationState::Attacking => {
            // 1st attack frame: use idle offset, 2nd attack frame: custom offsets
            let attack_frame = anim_frame % 2;
            if attack_frame == 0 {
                // 1st frame: same as idle
                if use_back { (-1.0, 1.0) } else { (0.0, 0.0) }
            } else {
                // 2nd frame: left 3, down 1 for left (mirrored for up); left 4, up 1 for down (mirrored for right)
                if use_back { (-3.0, 1.0) } else { (-4.0, -1.0) }
            }
        }
        AnimationState::Casting => (0.0, 0.0),
        AnimationState::ShootingBow => {
            // Same offsets as 2nd attack frame
            if use_back { (-3.0, 1.0) } else { (-4.0, -1.0) }
        }
        AnimationState::SittingChair => (0.0, 0.0),
        AnimationState::SittingGround => (0.0, 0.0),
    };

    // Invert x offset when flipped
    let adjusted_state_x = if should_flip_horizontal(direction) {
        -state_x
    } else {
        state_x
    };

    (base_x + adjusted_state_x, base_y + state_y)
}

// ============================================================================
// Body Armor Animation System (single-row sprite format)
// ============================================================================

/// Result of body armor frame calculation
#[derive(Debug, Clone, Copy)]
pub struct BodyArmorFrameResult {
    /// Frame index (0-based) in the single-row spritesheet
    pub frame: u32,
    /// Whether to flip the sprite horizontally
    pub flip_h: bool,
}

/// Get the body armor frame index for the current animation state and direction
///
/// Body armor sprite sheet layout (0-indexed, 22 frames total):
/// - 0-1: Standing front/back
/// - 2-5: Walking front (Down/Right)
/// - 6-9: Walking back (Up/Left)
/// - 10-11: Magic front/back
/// - 12-15: Attack (4 frames)
/// - 16-17: Sitting in chair front/back
/// - 18-19: Sitting on ground front/back
/// - 20-21: Archery front/back
pub fn get_body_armor_frame(state: AnimationState, direction: Direction, anim_frame: u32) -> BodyArmorFrameResult {
    let use_back = is_up_or_left_direction(direction);
    let flip_h = should_flip_horizontal(direction);

    let frame = match state {
        AnimationState::Idle => {
            if use_back { 1 } else { 0 }
        }
        AnimationState::Walking => {
            let frame_in_walk = anim_frame % 4;
            if use_back {
                6 + frame_in_walk // Frames 6-9
            } else {
                2 + frame_in_walk // Frames 2-5
            }
        }
        AnimationState::Attacking => {
            // Attack frame 1: use idle frame, Attack frame 2: use attack frame
            let attack_frame = anim_frame % 2;
            if attack_frame == 0 {
                // Frame 1: use idle
                if use_back { 1 } else { 0 }
            } else {
                // Frame 2: use attack (13 for front, 15 for back)
                if use_back { 15 } else { 13 }
            }
        }
        AnimationState::Casting => {
            // Magic frames 10-11
            if use_back { 11 } else { 10 }
        }
        AnimationState::ShootingBow => {
            // Archery frames 20-21
            if use_back { 21 } else { 20 }
        }
        AnimationState::SittingChair => {
            // Sitting chair frames 16-17
            if use_back { 17 } else { 16 }
        }
        AnimationState::SittingGround => {
            // Sitting ground frames 18-19
            if use_back { 19 } else { 18 }
        }
    };

    BodyArmorFrameResult { frame, flip_h }
}

/// Get the pixel offset for body armor positioning relative to the player sprite
///
/// Body armor covers the torso and should align with the player's body in each animation frame.
/// These offsets are similar to boots but positioned higher to cover the torso.
pub fn get_body_armor_offset(state: AnimationState, direction: Direction, anim_frame: u32) -> (f32, f32) {
    let use_back = is_up_or_left_direction(direction);

    // Base offset: body armor is 34 wide (same as player's 34)
    // Body armor is 77 tall, player is 78, nearly same height
    let base_x = 0.0;   // Same width as player, no centering needed
    let base_y = 0.0;   // Start at top of player sprite

    // Per-state offsets for alignment (similar to boots but for torso)
    let (state_x, state_y) = match state {
        AnimationState::Idle => {
            // Up/Left: right 1px for up (mirrored left for left), up 2px
            // Down/Right: up 3px (mirrored)
            if use_back { (-1.0, -2.0) } else { (0.0, -3.0) }
        }
        AnimationState::Walking => {
            // Up/Left: right 1px for up (mirrored left for left), up 3px
            // Down/Right: up 4px (mirrored)
            if use_back { (-1.0, -3.0) } else { (0.0, -4.0) }
        }
        AnimationState::Attacking => {
            let attack_frame = anim_frame % 2;
            if attack_frame == 0 {
                // Frame 1: right 1px for up (mirrored left for left), up 2px
                // Down/Right: up 3px (mirrored)
                if use_back { (-1.0, -2.0) } else { (0.0, -3.0) }
            } else {
                // Frame 2:
                // Up/Left: right 4px, up 2px for up (mirrored for left)
                // Down/Right: left 2px, up 3px (mirrored for right)
                if use_back { (-4.0, -2.0) } else { (-2.0, -3.0) }
            }
        }
        AnimationState::Casting => (0.0, 0.0),
        AnimationState::ShootingBow => {
            // Up/Left: right 1px for up (mirrored left for left), up 2px
            // Down/Right: left 4px, up 3px (mirrored for right)
            if use_back { (-1.0, -2.0) } else { (-4.0, -3.0) }
        }
        AnimationState::SittingChair => (0.0, 0.0),
        AnimationState::SittingGround => (0.0, 0.0),
    };

    // Invert x offset when flipped
    let adjusted_state_x = if should_flip_horizontal(direction) {
        -state_x
    } else {
        state_x
    };

    (base_x + adjusted_state_x, base_y + state_y)
}

// ============================================================================
// Head Equipment Animation System (simple 2-frame format: front/back)
// ============================================================================

/// Result of head frame calculation
#[derive(Debug, Clone, Copy)]
pub struct HeadFrameResult {
    /// Frame index (0 = front, 1 = back)
    pub frame: u32,
    /// Whether to flip the sprite horizontally
    pub flip_h: bool,
}

/// Get the head equipment frame index for the current direction
///
/// Head sprite sheet layout (0-indexed, 2 frames total):
/// - 0: Front-facing (Down, Right directions)
/// - 1: Back-facing (Up, Left directions)
pub fn get_head_frame(direction: Direction) -> HeadFrameResult {
    let use_back = is_up_or_left_direction(direction);
    let flip_h = should_flip_horizontal(direction);

    HeadFrameResult {
        frame: if use_back { 1 } else { 0 },
        flip_h,
    }
}

/// Get the pixel offset for head equipment positioning relative to the player sprite
///
/// Head equipment is positioned at the top of the player sprite and follows the same
/// offset patterns as hair during animations.
pub fn get_head_offset(state: AnimationState, direction: Direction, anim_frame: u32) -> (f32, f32) {
    let use_back = is_up_or_left_direction(direction);

    // Base offset: head is 30 wide, player is 34, center it
    // Head starts at top of player
    let base_x = 2.0;  // (34 - 30) / 2 = 2
    let base_y = -3.0; // Align with top of head, same as hair

    // Per-state offsets (mirroring hair offsets)
    let (state_x, state_y) = match state {
        AnimationState::Idle => {
            if use_back { (-2.0, 0.0) } else { (-1.0, 0.0) }
        }
        AnimationState::Walking => {
            if use_back { (-2.0, 0.0) } else { (-1.0, 0.0) }
        }
        AnimationState::Attacking => {
            let attack_frame = anim_frame % 2;
            if attack_frame == 0 {
                // Frame 1: same as idle
                if use_back { (-2.0, 0.0) } else { (-1.0, 0.0) }
            } else {
                // Frame 2: more dramatic shift (same as hair attack offsets)
                if use_back { (-5.0, -2.0) } else { (-6.0, 2.0) }
            }
        }
        AnimationState::Casting => (0.0, 0.0),
        AnimationState::ShootingBow => {
            if use_back { (-1.0, -3.0) } else { (-2.0, -3.0) }
        }
        AnimationState::SittingChair => (0.0, 0.0),
        AnimationState::SittingGround => (0.0, 0.0),
    };

    // Invert x offset when flipped
    let adjusted_state_x = if should_flip_horizontal(direction) {
        -state_x
    } else {
        state_x
    };

    (base_x + adjusted_state_x, base_y + state_y)
}
