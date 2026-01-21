use crate::game::Direction;
pub const SPRITE_WIDTH: f32 = 34.0;
pub const SPRITE_HEIGHT: f32 = 78.0;

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
