//! Generic scrollable list utilities

use macroquad::prelude::*;

/// Configuration for a scrollable list
pub struct ScrollableListConfig {
    /// Total height of the visible area
    pub visible_height: f32,
    /// Height of each item in the list
    pub item_height: f32,
    /// Spacing between items
    pub item_spacing: f32,
    /// Total number of items in the list
    pub total_items: usize,
    /// Current scroll offset in pixels
    pub scroll_offset: f32,
}

/// Result of scrollable list calculations
pub struct ScrollableListState {
    /// First visible item index
    pub first_visible: usize,
    /// Last visible item index (exclusive)
    pub last_visible: usize,
    /// Y offset to apply to the first visible item
    pub first_item_offset: f32,
    /// Maximum scroll offset (clamped)
    pub max_scroll: f32,
    /// Clamped scroll offset
    pub clamped_scroll: f32,
    /// Whether scrollbar should be shown
    pub show_scrollbar: bool,
    /// Scrollbar thumb position (0.0 to 1.0)
    pub scrollbar_position: f32,
    /// Scrollbar thumb size (0.0 to 1.0)
    pub scrollbar_size: f32,
}

impl ScrollableListConfig {
    /// Calculate the scrollable list state
    pub fn calculate(&self) -> ScrollableListState {
        let total_height = self.total_items as f32 * (self.item_height + self.item_spacing);
        let max_scroll = (total_height - self.visible_height).max(0.0);
        let clamped_scroll = self.scroll_offset.clamp(0.0, max_scroll);

        let show_scrollbar = total_height > self.visible_height;

        // Calculate visible range
        let item_total_height = self.item_height + self.item_spacing;
        let first_visible = (clamped_scroll / item_total_height).floor() as usize;
        let visible_count = (self.visible_height / item_total_height).ceil() as usize + 1;
        let last_visible = (first_visible + visible_count).min(self.total_items);

        // Offset for smooth scrolling (partial item visibility)
        let first_item_offset = -(clamped_scroll % item_total_height);

        // Scrollbar calculations
        let scrollbar_size = if total_height > 0.0 {
            (self.visible_height / total_height).min(1.0)
        } else {
            1.0
        };
        let scrollbar_position = if max_scroll > 0.0 {
            clamped_scroll / max_scroll
        } else {
            0.0
        };

        ScrollableListState {
            first_visible,
            last_visible,
            first_item_offset,
            max_scroll,
            clamped_scroll,
            show_scrollbar,
            scrollbar_position,
            scrollbar_size,
        }
    }
}

/// Handle mouse wheel scrolling for a list
/// Returns the new scroll offset
pub fn handle_scroll(
    current_scroll: f32,
    max_scroll: f32,
    scroll_speed: f32,
) -> f32 {
    let (_wheel_x, wheel_y) = mouse_wheel();
    let new_scroll = current_scroll - wheel_y * scroll_speed;
    new_scroll.clamp(0.0, max_scroll)
}

/// Check if a point is within a rectangle (for scroll area detection)
pub fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

/// Draw a simple scrollbar
pub fn draw_scrollbar(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    position: f32,
    thumb_size: f32,
    track_color: Color,
    thumb_color: Color,
) {
    // Draw track
    draw_rectangle(x, y, width, height, track_color);

    // Draw thumb
    let thumb_height = height * thumb_size;
    let thumb_y = y + (height - thumb_height) * position;
    draw_rectangle(x, thumb_y, width, thumb_height, thumb_color);
}
