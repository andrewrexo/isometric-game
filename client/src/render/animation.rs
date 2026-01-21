use crate::game::Direction;
pub const SPRITE_WIDTH: f32 = 34.0;
pub const SPRITE_HEIGHT: f32 = 78.0;

// Weapon sprite dimensions (single row, 17-18 frames per weapon)
pub const WEAPON_SPRITE_WIDTH: f32 = 68.0;
pub const WEAPON_SPRITE_HEIGHT: f32 = 84.0;

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
        AnimationState::Attacking => AnimationConfig::new(2, 2, 6.0, false, true),
        // Sitting chair on row 2, frames 4-5
        AnimationState::SittingChair => AnimationConfig::new(2, 2, 1.0, true, false),
        // Sitting ground on row 3, frames 0-1
        AnimationState::SittingGround => AnimationConfig::new(3, 2, 1.0, true, false),
        // Casting on row 1, frames 4-5
        AnimationState::Casting => AnimationConfig::new(1, 2, 6.0, false, false),
        // Shooting bow on row 3, frames 2-3
        AnimationState::ShootingBow => AnimationConfig::new(3, 2, 8.0, false, false),
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
                // Row 3, frames 2-3
                (3, 2 + frame_index)
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
            // Use frame 17 for ranged (if available), otherwise fall back to attack
            if use_back { (15, None) } else { (17, None) }
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
        AnimationState::ShootingBow => (0.0, 0.0),
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
