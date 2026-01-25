use macroquad::prelude::*;
use std::collections::HashSet;
use crate::game::{GameState, ContextMenu, ContextMenuTarget, DragState, DragSource, GoldDropDialog, PathState, pathfinding};
use crate::render::animation::AnimationState;
use crate::render::isometric::screen_to_world;
use crate::ui::{UiElementId, UiLayout};
use crate::network::messages::ClientMessage;
use crate::audio::AudioManager;

/// Build set of tiles occupied by entities (other players + NPCs) for pathfinding
fn build_occupied_set(state: &GameState) -> HashSet<(i32, i32)> {
    let mut occupied = HashSet::new();

    // When in interior mode, don't count overworld players as obstacles
    // (they shouldn't be in our instance anyway)
    let in_interior = state.current_interior.is_some();

    // Add other players (not local player)
    // Skip if in interior - we'll only see players in our instance from server updates
    if !in_interior {
        for (id, player) in &state.players {
            if state.local_player_id.as_ref() == Some(id) {
                continue;
            }
            if !player.is_dead {
                occupied.insert((player.x.round() as i32, player.y.round() as i32));
            }
        }
    }

    // Add all alive NPCs
    for npc in state.npcs.values() {
        if npc.is_alive() {
            occupied.insert((npc.x.round() as i32, npc.y.round() as i32));
        }
    }

    occupied
}

/// Input commands that can be sent to the server
#[derive(Debug, Clone)]
pub enum InputCommand {
    Move { dx: f32, dy: f32 },
    Face { direction: u8 },
    Attack,
    Target { entity_id: String },
    ClearTarget,
    Chat { text: String },
    Pickup { item_id: String },
    UseItem { slot_index: u8 },
    // Quest commands
    Interact { npc_id: String },
    DialogueChoice { quest_id: String, choice_id: String },
    CloseDialogue,
    // Crafting commands
    Craft { recipe_id: String },
    // Equipment commands
    Equip { slot_index: u8 },
    Unequip { slot_type: String, target_slot: Option<u8> },
    // Inventory commands
    DropItem { slot_index: u8, quantity: u32, target_x: Option<i32>, target_y: Option<i32> },
    DropGold { amount: i32 },
    SwapSlots { from_slot: u8, to_slot: u8 },
    // Shop commands
    ShopBuy { npc_id: String, item_id: String, quantity: u32 },
    ShopSell { npc_id: String, item_id: String, quantity: u32 },
    // Portal commands
    EnterPortal { portal_id: String },
}

/// Cardinal directions for isometric movement (no diagonals)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CardinalDir {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl CardinalDir {
    /// Convert to server direction enum value (matches Direction enum)
    fn to_direction_u8(self) -> u8 {
        match self {
            CardinalDir::Down => 0,
            CardinalDir::Left => 1,
            CardinalDir::Up => 2,
            CardinalDir::Right => 3,
            CardinalDir::None => 0, // Default to down
        }
    }
}

/// Threshold for distinguishing face vs move (in seconds)
const FACE_THRESHOLD: f64 = 0.05; // 100ms

pub struct InputHandler {
    // Track last sent velocity to detect changes
    last_dx: f32,
    last_dy: f32,
    // Track which direction was pressed first (for priority)
    current_dir: CardinalDir,
    // Track previous direction for detecting key release
    prev_dir: CardinalDir,
    // Send commands at server tick rate
    last_send_time: f64,
    send_interval: f64,
    // Attack cooldown tracking (matches server cooldown)
    last_attack_time: f64,
    attack_cooldown: f64,
    // Track when current direction key was pressed (for face vs move)
    dir_press_time: f64,
    // Track if we've sent a move command for the current key press
    move_sent: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            last_dx: 0.0,
            last_dy: 0.0,
            current_dir: CardinalDir::None,
            prev_dir: CardinalDir::None,
            last_send_time: 0.0,
            send_interval: 0.05, // 50ms = 20Hz (matches server tick rate)
            last_attack_time: 0.0,
            attack_cooldown: 0.8, // 800 ms (matches server ATTACK_COOLDOWN_MS)
            dir_press_time: 0.0,
            move_sent: false,
        }
    }

    pub fn process(&mut self, state: &mut GameState, layout: &UiLayout, audio: &mut AudioManager) -> Vec<InputCommand> {
        let mut commands = Vec::new();
        let current_time = get_time();

        // Get current mouse position
        let (mx, my) = mouse_position();

        // Update hover state for visual feedback (used by renderer next frame)
        state.ui_state.hovered_element = layout.hit_test(mx, my).cloned();

        // Update hovered tile based on mouse position (only when not hovering UI)
        // Use round() instead of floor() because tile sprites are visually centered
        // at integer world coordinates, forming diamonds that span [-0.5, 0.5) around each point
        if state.ui_state.hovered_element.is_none() {
            let (world_x, world_y) = screen_to_world(mx, my, &state.camera);
            let tile_x = world_x.round() as i32;
            let tile_y = world_y.round() as i32;
            state.hovered_tile = Some((tile_x, tile_y));

            // Check for entity hover (players and NPCs within hover radius)
            let hover_radius = 0.6; // World units - slightly larger than tile size for easier targeting
            let mut hovered_entity: Option<String> = None;

            // Check NPCs first (they're usually more important to interact with)
            for npc in state.npcs.values() {
                if npc.state != crate::game::npc::NpcState::Dead {
                    let dx = world_x - npc.x;
                    let dy = world_y - npc.y;
                    if dx * dx + dy * dy < hover_radius * hover_radius {
                        hovered_entity = Some(npc.id.clone());
                        break;
                    }
                }
            }

            // Check players if no NPC is hovered
            if hovered_entity.is_none() {
                for player in state.players.values() {
                    if !player.is_dead {
                        let dx = world_x - player.x;
                        let dy = world_y - player.y;
                        if dx * dx + dy * dy < hover_radius * hover_radius {
                            hovered_entity = Some(player.id.clone());
                            break;
                        }
                    }
                }
            }

            state.hovered_entity_id = hovered_entity;
        } else {
            state.hovered_tile = None;
            state.hovered_entity_id = None;
        }

        // For click detection, do a fresh hit-test at the moment of click
        // This ensures we detect what's actually under the mouse when clicked
        let mouse_clicked = is_mouse_button_pressed(MouseButton::Left);
        let mouse_right_clicked = is_mouse_button_pressed(MouseButton::Right);
        let mouse_released = is_mouse_button_released(MouseButton::Left);
        let clicked_element = if mouse_clicked || mouse_right_clicked || mouse_released {
            layout.hit_test(mx, my).cloned()
        } else {
            None
        };

        // Toggle debug mode
        if is_key_pressed(KeyCode::F3) {
            // Debug toggle handled in main loop
        }

        // Handle drag and drop for inventory slot rearrangement and equipment
        if mouse_released {
            if let Some(drag) = state.ui_state.drag_state.take() {
                // Drag completed - check if we're over a valid drop target
                if let Some(ref element) = clicked_element {
                    match element {
                        UiElementId::InventorySlot(to_idx) | UiElementId::QuickSlot(to_idx) => {
                            match &drag.source {
                                DragSource::Inventory(from_idx) => {
                                    // Swap inventory slots if dropping on a different slot
                                    if *from_idx != *to_idx {
                                        // Optimistic update: immediately swap locally
                                        state.inventory.swap_slots(*from_idx, *to_idx);
                                        audio.play_sfx("item_put");

                                        commands.push(InputCommand::SwapSlots {
                                            from_slot: *from_idx as u8,
                                            to_slot: *to_idx as u8,
                                        });
                                    }
                                }
                                DragSource::Equipment(slot_type) => {
                                    // Dragging from equipment to inventory - unequip to specific slot
                                    // Optimistic update: immediately move to inventory and clear equipment
                                    if state.inventory.slots.get(*to_idx).map(|s| s.is_none()).unwrap_or(false) {
                                        state.inventory.set_slot(*to_idx, drag.item_id.clone(), drag.quantity);

                                        // Update player's equipped state optimistically
                                        if let Some(local_id) = &state.local_player_id.clone() {
                                            if let Some(player) = state.players.get_mut(local_id) {
                                                match slot_type.as_str() {
                                                    "head" => player.equipped_head = None,
                                                    "body" => player.equipped_body = None,
                                                    "weapon" => player.equipped_weapon = None,
                                                    "back" => player.equipped_back = None,
                                                    "feet" => player.equipped_feet = None,
                                                    "ring" => player.equipped_ring = None,
                                                    "gloves" => player.equipped_gloves = None,
                                                    "necklace" => player.equipped_necklace = None,
                                                    "belt" => player.equipped_belt = None,
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }

                                    audio.play_sfx("item_put");
                                    commands.push(InputCommand::Unequip {
                                        slot_type: slot_type.clone(),
                                        target_slot: Some(*to_idx as u8),
                                    });
                                }
                            }
                        }
                        UiElementId::EquipmentSlot(target_slot_type) => {
                            match &drag.source {
                                DragSource::Inventory(from_idx) => {
                                    // Dragging from inventory to equipment - equip if valid slot type
                                    // First check if player meets requirements before optimistic update
                                    let item_def = state.item_registry.get_or_placeholder(&drag.item_id);
                                    let can_equip = if let Some(ref equip) = item_def.equipment {
                                        // Check slot type matches target
                                        let slot_matches = equip.slot_type == *target_slot_type;
                                        // Check level requirement - combat level covers all requirements
                                        let level_required = equip.attack_level_required.max(equip.defence_level_required);
                                        let level_ok = state.get_local_player()
                                            .map(|p| p.skills.combat.level >= level_required)
                                            .unwrap_or(false);
                                        slot_matches && level_ok
                                    } else {
                                        false // Not equippable
                                    };

                                    if can_equip {
                                        // Optimistic update: immediately equip and clear inventory slot
                                        if let Some(local_id) = &state.local_player_id.clone() {
                                            if let Some(player) = state.players.get_mut(local_id) {
                                                match target_slot_type.as_str() {
                                                    "head" => player.equipped_head = Some(drag.item_id.clone()),
                                                    "body" => player.equipped_body = Some(drag.item_id.clone()),
                                                    "weapon" => player.equipped_weapon = Some(drag.item_id.clone()),
                                                    "back" => player.equipped_back = Some(drag.item_id.clone()),
                                                    "feet" => player.equipped_feet = Some(drag.item_id.clone()),
                                                    "ring" => player.equipped_ring = Some(drag.item_id.clone()),
                                                    "gloves" => player.equipped_gloves = Some(drag.item_id.clone()),
                                                    "necklace" => player.equipped_necklace = Some(drag.item_id.clone()),
                                                    "belt" => player.equipped_belt = Some(drag.item_id.clone()),
                                                    _ => {}
                                                }
                                            }
                                        }
                                        state.inventory.clear_slot(*from_idx);
                                        audio.play_sfx("item_put");

                                        commands.push(InputCommand::Equip { slot_index: *from_idx as u8 });
                                    }
                                    // If can't equip, drag is cancelled - item stays in inventory
                                }
                                DragSource::Equipment(source_slot_type) => {
                                    // Dragging from equipment slot to another equipment slot
                                    // Only makes sense if they're different types, otherwise no-op
                                    if source_slot_type != target_slot_type {
                                        // Can't swap different equipment slot types directly
                                        // Would need unequip + equip, which isn't supported
                                    }
                                }
                            }
                        }
                        _ => {
                            // Dropped on non-inventory UI element, cancel drag
                        }
                    }
                } else {
                    // No UI element under cursor - check for world tile drop
                    // Use the already-computed hovered_tile for consistency with visual feedback
                    if let DragSource::Inventory(from_idx) = &drag.source {
                        if let Some((tile_x, tile_y)) = state.hovered_tile {
                            // Get player position
                            if let Some(player) = state.get_local_player() {
                                let player_x = player.x.round() as i32;
                                let player_y = player.y.round() as i32;

                                // Check Chebyshev distance (must be exactly 1 - adjacent/diagonal)
                                let dx = (tile_x - player_x).abs();
                                let dy = (tile_y - player_y).abs();
                                let is_adjacent = dx <= 1 && dy <= 1 && (dx > 0 || dy > 0);

                                if is_adjacent {
                                    // Check for Ctrl/Cmd modifier for single item drop
                                    let ctrl_held = is_key_down(KeyCode::LeftControl)
                                        || is_key_down(KeyCode::RightControl)
                                        || is_key_down(KeyCode::LeftSuper)
                                        || is_key_down(KeyCode::RightSuper);

                                    let quantity = if ctrl_held { 1 } else { drag.quantity as u32 };

                                    commands.push(InputCommand::DropItem {
                                        slot_index: *from_idx as u8,
                                        quantity,
                                        target_x: Some(tile_x),
                                        target_y: Some(tile_y),
                                    });
                                    audio.play_sfx("item_put");
                                }
                            }
                        }
                    }
                    // Equipment drag to world is not supported - just cancel
                }
                // Drag ended (either completed swap or cancelled)
                return commands;
            }
        }

        // Double-click detection threshold (300ms)
        const DOUBLE_CLICK_THRESHOLD: f64 = 0.3;

        // Start drag on left click on inventory slot with item
        // But first check for double-click to equip
        if mouse_clicked && state.ui_state.drag_state.is_none() {
            if let Some(ref element) = clicked_element {
                match element {
                    UiElementId::InventorySlot(idx) | UiElementId::QuickSlot(idx) => {
                        // Check if slot has an item
                        if let Some(Some(slot)) = state.inventory.slots.get(*idx) {
                            // Check for shift+click to drop (if enabled)
                            let shift_held = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                            if shift_held && state.ui_state.shift_drop_enabled {
                                // Drop the entire stack at player position
                                commands.push(InputCommand::DropItem {
                                    slot_index: *idx as u8,
                                    quantity: slot.quantity as u32,
                                    target_x: None,
                                    target_y: None,
                                });
                                audio.play_sfx("item_put");
                                return commands;
                            }

                            // Check for double-click
                            let is_double_click = state.ui_state.double_click_state.last_click_slot == Some(*idx)
                                && current_time - state.ui_state.double_click_state.last_click_time < DOUBLE_CLICK_THRESHOLD;

                            if is_double_click {
                                // Reset double-click state
                                state.ui_state.double_click_state.last_click_slot = None;
                                state.ui_state.double_click_state.last_click_time = 0.0;

                                // Check if item is equippable
                                let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                                if item_def.equipment.is_some() {
                                    // Equip the item
                                    commands.push(InputCommand::Equip { slot_index: *idx as u8 });
                                    return commands;
                                } else {
                                    // Not equippable - use the item instead (e.g., health potion)
                                    commands.push(InputCommand::UseItem { slot_index: *idx as u8 });
                                    return commands;
                                }
                            } else {
                                // First click - record for potential double-click
                                state.ui_state.double_click_state.last_click_slot = Some(*idx);
                                state.ui_state.double_click_state.last_click_time = current_time;

                                // Start drag from inventory
                                state.ui_state.drag_state = Some(DragState {
                                    source: DragSource::Inventory(*idx),
                                    item_id: slot.item_id.clone(),
                                    quantity: slot.quantity,
                                });
                                audio.play_sfx("item_grab");
                                // Don't process other input while starting drag
                                return commands;
                            }
                        }
                    }
                    UiElementId::EquipmentSlot(slot_type) => {
                        // Check if equipment slot has an item
                        let equipped_item = match slot_type.as_str() {
                            "head" => state.get_local_player().and_then(|p| p.equipped_head.clone()),
                            "body" => state.get_local_player().and_then(|p| p.equipped_body.clone()),
                            "weapon" => state.get_local_player().and_then(|p| p.equipped_weapon.clone()),
                            "back" => state.get_local_player().and_then(|p| p.equipped_back.clone()),
                            "feet" => state.get_local_player().and_then(|p| p.equipped_feet.clone()),
                            "ring" => state.get_local_player().and_then(|p| p.equipped_ring.clone()),
                            "gloves" => state.get_local_player().and_then(|p| p.equipped_gloves.clone()),
                            "necklace" => state.get_local_player().and_then(|p| p.equipped_necklace.clone()),
                            "belt" => state.get_local_player().and_then(|p| p.equipped_belt.clone()),
                            _ => None,
                        };
                        if let Some(item_id) = equipped_item {
                            // Start drag from equipment slot
                            state.ui_state.drag_state = Some(DragState {
                                source: DragSource::Equipment(slot_type.clone()),
                                item_id,
                                quantity: 1, // Equipment is always quantity 1
                            });
                            audio.play_sfx("item_grab");
                            return commands;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Handle context menu interactions first
        if let Some(ref menu) = state.ui_state.context_menu {
            // Auto-hide context menu when mouse leaves its bounds
            let padding = 8.0;
            let header_height = 24.0;
            let option_height = 28.0;
            let menu_width = 120.0;

            // Calculate number of options (same logic as render_context_menu)
            let num_options = match &menu.target {
                ContextMenuTarget::EquipmentSlot(_) => 1, // Unequip only
                ContextMenuTarget::Gold => 1, // Drop only
                ContextMenuTarget::InventorySlot(slot_index) => {
                    let is_equippable = state.inventory.slots.get(*slot_index)
                        .and_then(|s| s.as_ref())
                        .map(|slot| {
                            let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                            item_def.equipment.is_some()
                        })
                        .unwrap_or(false);
                    if is_equippable { 2 } else { 1 } // Equip+Drop or just Drop
                }
            };

            let content_height = num_options as f32 * option_height + padding;
            let menu_height = header_height + content_height + padding;

            // Apply same screen clamping as render_context_menu
            let mut menu_x = menu.x.floor();
            let mut menu_y = menu.y.floor();

            let screen_w = screen_width();
            let screen_h = screen_height();

            if menu_x + menu_width > screen_w {
                menu_x = (screen_w - menu_width - 5.0).floor();
            }
            if menu_y + menu_height > screen_h {
                menu_y = (screen_h - menu_height - 5.0).floor();
            }

            // Add some margin for easier interaction
            let margin = 4.0;
            let is_mouse_inside = mx >= menu_x - margin
                && mx <= menu_x + menu_width + margin
                && my >= menu_y - margin
                && my <= menu_y + menu_height + margin;

            if !is_mouse_inside {
                state.ui_state.context_menu = None;
            }
        }

        if state.ui_state.context_menu.is_some() {
            if let Some(ref element) = clicked_element {
                if mouse_clicked {
                    match element {
                        UiElementId::ContextMenuOption(option_idx) => {
                            // Get menu info before clearing it
                            let menu = state.ui_state.context_menu.take().unwrap();

                            match &menu.target {
                                ContextMenuTarget::EquipmentSlot(slot_type) => {
                                    // Equipment slot context menu - only unequip option
                                    if *option_idx == 0 {
                                        commands.push(InputCommand::Unequip {
                                            slot_type: slot_type.clone(),
                                            target_slot: None, // Use first available slot
                                        });
                                    }
                                }
                                ContextMenuTarget::Gold => {
                                    // Gold context menu - only drop option
                                    if *option_idx == 0 {
                                        // Open gold drop dialog
                                        state.ui_state.gold_drop_dialog = Some(GoldDropDialog {
                                            input: String::new(),
                                            cursor: 0,
                                        });
                                    }
                                }
                                ContextMenuTarget::InventorySlot(slot_index) => {
                                    // Inventory slot context menu
                                    // Check if item is equippable to determine option indices
                                    let is_equippable = state.inventory.slots.get(*slot_index)
                                        .and_then(|s| s.as_ref())
                                        .map(|slot| {
                                            let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                                            item_def.equipment.is_some()
                                        })
                                        .unwrap_or(false);

                                    if is_equippable {
                                        // Options: Equip (0), Drop (1)
                                        match option_idx {
                                            0 => commands.push(InputCommand::Equip { slot_index: *slot_index as u8 }),
                                            1 => {
                                                if let Some(slot) = state.inventory.slots.get(*slot_index).and_then(|s| s.as_ref()) {
                                                    commands.push(InputCommand::DropItem { slot_index: *slot_index as u8, quantity: slot.quantity as u32, target_x: None, target_y: None });
                                                }
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        // Options: Drop (0) only
                                        if *option_idx == 0 {
                                            if let Some(slot) = state.inventory.slots.get(*slot_index).and_then(|s| s.as_ref()) {
                                                commands.push(InputCommand::DropItem { slot_index: *slot_index as u8, quantity: slot.quantity as u32, target_x: None, target_y: None });
                                            }
                                        }
                                    }
                                }
                            }
                            return commands;
                        }
                        _ => {
                            // Clicked somewhere else, close menu
                            state.ui_state.context_menu = None;
                        }
                    }
                }
            } else if mouse_clicked || mouse_right_clicked {
                // Clicked outside any element, close menu
                state.ui_state.context_menu = None;
            }

            // Escape closes context menu
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.context_menu = None;
                return commands;
            }
        }

        // Handle menu button clicks (always visible, handle before modal UIs)
        if mouse_clicked {
            if let Some(ref element) = clicked_element {
                match element {
                    UiElementId::MenuButtonCharacter => {
                        audio.play_sfx("enter");
                        // Toggle character panel, close others if opening
                        if state.ui_state.character_open {
                            state.ui_state.character_open = false;
                        } else {
                            state.ui_state.character_open = true;
                            state.ui_state.inventory_open = false;
                            state.ui_state.character_panel_open = false;
                            state.ui_state.social_open = false;
                            state.ui_state.skills_open = false;
                        }
                        return commands;
                    }
                    UiElementId::MenuButtonInventory => {
                        audio.play_sfx("enter");
                        // Toggle inventory panel, close others if opening
                        if state.ui_state.inventory_open {
                            state.ui_state.inventory_open = false;
                        } else {
                            state.ui_state.inventory_open = true;
                            state.ui_state.character_panel_open = false;
                            state.ui_state.character_open = false;
                            state.ui_state.social_open = false;
                            state.ui_state.skills_open = false;
                        }
                        return commands;
                    }
                    UiElementId::MenuButtonCharacter => {
                        audio.play_sfx("enter");
                        // Toggle character panel, close others if opening
                        if state.ui_state.character_panel_open {
                            state.ui_state.character_panel_open = false;
                        } else {
                            state.ui_state.character_panel_open = true;
                            state.ui_state.inventory_open = false;
                            state.ui_state.character_open = false;
                            state.ui_state.social_open = false;
                            state.ui_state.skills_open = false;
                        }
                        return commands;
                    }
                    UiElementId::MenuButtonSocial => {
                        audio.play_sfx("enter");
                        // Toggle social panel, close others if opening
                        if state.ui_state.social_open {
                            state.ui_state.social_open = false;
                        } else {
                            state.ui_state.social_open = true;
                            state.ui_state.inventory_open = false;
                            state.ui_state.character_panel_open = false;
                            state.ui_state.character_open = false;
                            state.ui_state.skills_open = false;
                        }
                        return commands;
                    }
                    UiElementId::MenuButtonSkills => {
                        audio.play_sfx("enter");
                        // Toggle skills panel, close others if opening
                        if state.ui_state.skills_open {
                            state.ui_state.skills_open = false;
                        } else {
                            state.ui_state.skills_open = true;
                            state.ui_state.inventory_open = false;
                            state.ui_state.character_panel_open = false;
                            state.ui_state.character_open = false;
                            state.ui_state.social_open = false;
                        }
                        return commands;
                    }
                    UiElementId::MenuButtonSettings => {
                        audio.play_sfx("enter");
                        // Toggle escape/settings menu
                        state.ui_state.escape_menu_open = !state.ui_state.escape_menu_open;
                        return commands;
                    }
                    _ => {}
                }
            }
        }

        // Handle escape menu
        if state.ui_state.escape_menu_open {
            // Handle mouse clicks on escape menu elements
            if let Some(ref element) = clicked_element {
                if mouse_clicked {
                    match element {
                        UiElementId::EscapeMenuZoom1x => {
                            audio.play_sfx("enter");
                            state.camera.zoom = 1.0;
                            state.ui_state.escape_menu_open = false;
                            return commands;
                        }
                        UiElementId::EscapeMenuZoom2x => {
                            audio.play_sfx("enter");
                            state.camera.zoom = 2.0;
                            state.ui_state.escape_menu_open = false;
                            return commands;
                        }
                        UiElementId::EscapeMenuMusicSlider => {
                            // Calculate volume from click position within slider
                            if let Some(slider_elem) = layout.elements.iter().find(|e| e.id == UiElementId::EscapeMenuMusicSlider) {
                                let (mouse_x, _) = mouse_position();
                                let relative_x = mouse_x - slider_elem.bounds.x;
                                let volume = (relative_x / slider_elem.bounds.w).clamp(0.0, 1.0);
                                state.ui_state.audio_volume = volume;
                                audio.set_music_volume(volume);
                            }
                            return commands;
                        }
                        UiElementId::EscapeMenuSfxSlider => {
                            // Calculate SFX volume from click position within slider
                            if let Some(slider_elem) = layout.elements.iter().find(|e| e.id == UiElementId::EscapeMenuSfxSlider) {
                                let (mouse_x, _) = mouse_position();
                                let relative_x = mouse_x - slider_elem.bounds.x;
                                let volume = (relative_x / slider_elem.bounds.w).clamp(0.0, 1.0);
                                state.ui_state.audio_sfx_volume = volume;
                                audio.set_sfx_volume(volume);
                            }
                            return commands;
                        }
                        UiElementId::EscapeMenuMuteToggle => {
                            audio.play_sfx("enter");
                            state.ui_state.audio_muted = !state.ui_state.audio_muted;
                            audio.toggle_mute();
                            return commands;
                        }
                        UiElementId::EscapeMenuShiftDropToggle => {
                            audio.play_sfx("enter");
                            state.ui_state.shift_drop_enabled = !state.ui_state.shift_drop_enabled;
                            return commands;
                        }
                        UiElementId::EscapeMenuDisconnect => {
                            audio.play_sfx("enter");
                            state.disconnect_requested = true;
                            state.ui_state.escape_menu_open = false;
                            return commands;
                        }
                        _ => {
                            // Clicked somewhere else, close menu
                            state.ui_state.escape_menu_open = false;
                        }
                    }
                }
            } else if mouse_clicked {
                // Clicked outside any element, close menu
                state.ui_state.escape_menu_open = false;
            }

            // Escape closes escape menu
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.escape_menu_open = false;
                return commands;
            }

            // Don't process other input while escape menu is open
            return commands;
        }

        // Handle gold drop dialog
        if state.ui_state.gold_drop_dialog.is_some() {
            // Handle button clicks
            if mouse_clicked {
                if let Some(ref element) = clicked_element {
                    match element {
                        UiElementId::GoldDropConfirm => {
                            // Parse amount and validate
                            let dialog = state.ui_state.gold_drop_dialog.as_ref().unwrap();
                            if let Ok(amount) = dialog.input.parse::<i32>() {
                                if amount > 0 && amount <= state.inventory.gold {
                                    commands.push(InputCommand::DropGold { amount });
                                    state.ui_state.gold_drop_dialog = None;
                                }
                            }
                            return commands;
                        }
                        UiElementId::GoldDropCancel => {
                            state.ui_state.gold_drop_dialog = None;
                            return commands;
                        }
                        _ => {}
                    }
                }
            }

            // Handle keyboard input
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.gold_drop_dialog = None;
                return commands;
            }

            if is_key_pressed(KeyCode::Enter) {
                // Confirm with Enter key
                let dialog = state.ui_state.gold_drop_dialog.as_ref().unwrap();
                if let Ok(amount) = dialog.input.parse::<i32>() {
                    if amount > 0 && amount <= state.inventory.gold {
                        commands.push(InputCommand::DropGold { amount });
                        state.ui_state.gold_drop_dialog = None;
                    }
                }
                return commands;
            }

            // Number key input
            let number_keys = [
                (KeyCode::Key0, '0'), (KeyCode::Key1, '1'), (KeyCode::Key2, '2'),
                (KeyCode::Key3, '3'), (KeyCode::Key4, '4'), (KeyCode::Key5, '5'),
                (KeyCode::Key6, '6'), (KeyCode::Key7, '7'), (KeyCode::Key8, '8'),
                (KeyCode::Key9, '9'),
                (KeyCode::Kp0, '0'), (KeyCode::Kp1, '1'), (KeyCode::Kp2, '2'),
                (KeyCode::Kp3, '3'), (KeyCode::Kp4, '4'), (KeyCode::Kp5, '5'),
                (KeyCode::Kp6, '6'), (KeyCode::Kp7, '7'), (KeyCode::Kp8, '8'),
                (KeyCode::Kp9, '9'),
            ];

            for (key, digit) in &number_keys {
                if is_key_pressed(*key) {
                    let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                    // Limit input length (max 10 digits for gold amounts)
                    if dialog.input.len() < 10 {
                        dialog.input.insert(dialog.cursor, *digit);
                        dialog.cursor += 1;
                    }
                }
            }

            // Backspace
            if is_key_pressed(KeyCode::Backspace) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                if dialog.cursor > 0 {
                    dialog.input.remove(dialog.cursor - 1);
                    dialog.cursor -= 1;
                }
            }

            // Delete
            if is_key_pressed(KeyCode::Delete) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                if dialog.cursor < dialog.input.len() {
                    dialog.input.remove(dialog.cursor);
                }
            }

            // Left/Right arrow navigation
            if is_key_pressed(KeyCode::Left) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                if dialog.cursor > 0 {
                    dialog.cursor -= 1;
                }
            }
            if is_key_pressed(KeyCode::Right) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                if dialog.cursor < dialog.input.len() {
                    dialog.cursor += 1;
                }
            }

            // Home/End
            if is_key_pressed(KeyCode::Home) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                dialog.cursor = 0;
            }
            if is_key_pressed(KeyCode::End) {
                let dialog = state.ui_state.gold_drop_dialog.as_mut().unwrap();
                dialog.cursor = dialog.input.len();
            }

            // Drain character queue to prevent ghost characters
            while get_char_pressed().is_some() {}

            // Don't process other input while gold drop dialog is open
            return commands;
        }

        // Handle dialogue mode - intercept input when dialogue is open
        if let Some(dialogue) = &state.ui_state.active_dialogue {
            // Handle mouse clicks on dialogue elements
            if let Some(ref element) = clicked_element {
                match element {
                    UiElementId::DialogueChoice(idx) => {
                        if *idx < dialogue.choices.len() {
                            let choice = &dialogue.choices[*idx];
                            commands.push(InputCommand::DialogueChoice {
                                quest_id: dialogue.quest_id.clone(),
                                choice_id: choice.id.clone(),
                            });
                            return commands;
                        }
                    }
                    UiElementId::DialogueContinue => {
                        commands.push(InputCommand::DialogueChoice {
                            quest_id: dialogue.quest_id.clone(),
                            choice_id: "__continue__".to_string(),
                        });
                        return commands;
                    }
                    _ => {}
                }
            }

            if !dialogue.choices.is_empty() {
                // Dialogue with choices - Escape cancels, number keys select
                if is_key_pressed(KeyCode::Escape) {
                    commands.push(InputCommand::CloseDialogue);
                    state.ui_state.active_dialogue = None;
                    return commands;
                }

                // Number keys (1-4) select dialogue choices
                let choice_keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
                for (i, key) in choice_keys.iter().enumerate() {
                    if i < dialogue.choices.len() && is_key_pressed(*key) {
                        let choice = &dialogue.choices[i];
                        commands.push(InputCommand::DialogueChoice {
                            quest_id: dialogue.quest_id.clone(),
                            choice_id: choice.id.clone(),
                        });
                        // Don't clear dialogue here - wait for server response
                        return commands;
                    }
                }
            } else {
                // No choices - Escape, Enter, or Space to continue/close
                // Send __continue__ to server so Lua script can resume execution
                // Don't clear dialogue here - wait for server response (either new dialogue or close)
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                    commands.push(InputCommand::DialogueChoice {
                        quest_id: dialogue.quest_id.clone(),
                        choice_id: "__continue__".to_string(),
                    });
                    return commands;
                }
            }

            // Don't process other input while dialogue is open
            return commands;
        }

        // Handle crafting mode
        if state.ui_state.crafting_open {
            // Handle mouse clicks on crafting elements (only on mouse down, not release)
            if mouse_clicked {
                if let Some(ref element) = clicked_element {
                    match element {
                    UiElementId::MainTab(idx) => {
                        state.ui_state.shop_main_tab = *idx;
                        return commands;
                    }
                    UiElementId::CraftingCategoryTab(idx) => {
                        if *idx != state.ui_state.crafting_selected_category {
                            state.ui_state.crafting_selected_category = *idx;
                            state.ui_state.crafting_selected_recipe = 0;
                        }
                        return commands;
                    }
                    UiElementId::CraftingRecipeItem(idx) => {
                        state.ui_state.crafting_selected_recipe = *idx;
                        return commands;
                    }
                    UiElementId::CraftingButton => {
                        // Get unique categories from recipes
                        let categories: Vec<&str> = {
                            let mut cats: Vec<&str> = state.recipe_definitions.iter()
                                .map(|r| r.category.as_str())
                                .collect();
                            cats.sort();
                            cats.dedup();
                            cats
                        };
                        let current_category = categories.get(state.ui_state.crafting_selected_category).copied().unwrap_or("consumables");
                        let recipes_in_category: Vec<&crate::game::RecipeDefinition> = state.recipe_definitions.iter()
                            .filter(|r| r.category == current_category)
                            .collect();
                        if let Some(recipe) = recipes_in_category.get(state.ui_state.crafting_selected_recipe) {
                            log::info!("Crafting (click): {}", recipe.id);
                            commands.push(InputCommand::Craft { recipe_id: recipe.id.clone() });
                        }
                        return commands;
                    }
                    UiElementId::ShopBuyItem(idx) => {
                        state.ui_state.shop_selected_buy_index = *idx;
                        state.ui_state.shop_buy_quantity = 1;
                        return commands;
                    }
                    UiElementId::ShopSellItem(idx) => {
                        state.ui_state.shop_selected_sell_index = *idx;
                        state.ui_state.shop_sell_quantity = 1;
                        return commands;
                    }
                    UiElementId::ShopBuyQuantityMinus => {
                        if state.ui_state.shop_buy_quantity > 1 {
                            state.ui_state.shop_buy_quantity -= 1;
                        }
                        return commands;
                    }
                    UiElementId::ShopBuyQuantityPlus => {
                        state.ui_state.shop_buy_quantity += 1;
                        return commands;
                    }
                    UiElementId::ShopSellQuantityMinus => {
                        if state.ui_state.shop_sell_quantity > 1 {
                            state.ui_state.shop_sell_quantity -= 1;
                        }
                        return commands;
                    }
                    UiElementId::ShopSellQuantityPlus => {
                        state.ui_state.shop_sell_quantity += 1;
                        return commands;
                    }
                    UiElementId::ShopBuyConfirmButton => {
                        if let Some(ref shop_data) = state.ui_state.shop_data {
                            if let Some(ref npc_id) = state.ui_state.shop_npc_id {
                                if let Some(stock_item) = shop_data.stock.get(state.ui_state.shop_selected_buy_index) {
                                    audio.play_sfx("buy");
                                    commands.push(InputCommand::ShopBuy {
                                        npc_id: npc_id.clone(),
                                        item_id: stock_item.item_id.clone(),
                                        quantity: state.ui_state.shop_buy_quantity as u32,
                                    });
                                }
                            }
                        }
                        return commands;
                    }
                    UiElementId::ShopSellConfirmButton => {
                        if let Some(ref npc_id) = state.ui_state.shop_npc_id {
                            let inventory_items: Vec<_> = state.inventory.slots.iter()
                                .filter_map(|slot| slot.as_ref())
                                .collect();
                            if let Some(inv_slot) = inventory_items.get(state.ui_state.shop_selected_sell_index) {
                                commands.push(InputCommand::ShopSell {
                                    npc_id: npc_id.clone(),
                                    item_id: inv_slot.item_id.clone(),
                                    quantity: state.ui_state.shop_sell_quantity as u32,
                                });
                            }
                        }
                        return commands;
                    }
                        _ => {}
                    }
                }
            }

            // Escape closes crafting/shop menu
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.crafting_open = false;
                state.ui_state.crafting_npc_id = None;
                state.ui_state.shop_data = None;
                return commands;
            }

            // Q switches between Recipes/Shop main tabs
            if is_key_pressed(KeyCode::Q) {
                state.ui_state.shop_main_tab = if state.ui_state.shop_main_tab == 0 { 1 } else { 0 };
            }

            if state.ui_state.shop_main_tab == 0 {
                // Recipes tab keyboard controls
                // Get unique categories from recipes, merging consumables and materials
                let categories: Vec<String> = {
                    let mut cats: Vec<String> = state.recipe_definitions.iter()
                        .map(|r| {
                            if r.category == "materials" || r.category == "consumables" {
                                "supplies".to_string()
                            } else {
                                r.category.clone()
                            }
                        })
                        .collect();
                    cats.sort();
                    cats.dedup();
                    cats
                };

                // Left/Right navigate categories
                if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                    if state.ui_state.crafting_selected_category > 0 {
                        state.ui_state.crafting_selected_category -= 1;
                        state.ui_state.crafting_selected_recipe = 0;
                    }
                }
                if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                    if state.ui_state.crafting_selected_category < categories.len().saturating_sub(1) {
                        state.ui_state.crafting_selected_category += 1;
                        state.ui_state.crafting_selected_recipe = 0;
                    }
                }

                // Get recipes for current category
                let selected_idx = state.ui_state.crafting_selected_category.min(categories.len().saturating_sub(1));
                let current_category = categories.get(selected_idx).map(|s| s.as_str()).unwrap_or("supplies");
                let recipes_in_category: Vec<&crate::game::RecipeDefinition> = state.recipe_definitions.iter()
                    .filter(|r| {
                        if current_category == "supplies" {
                            r.category == "consumables" || r.category == "materials"
                        } else {
                            r.category == current_category
                        }
                    })
                    .collect();

                // Up/Down navigate recipes
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    if state.ui_state.crafting_selected_recipe > 0 {
                        state.ui_state.crafting_selected_recipe -= 1;
                    }
                }
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    if state.ui_state.crafting_selected_recipe < recipes_in_category.len().saturating_sub(1) {
                        state.ui_state.crafting_selected_recipe += 1;
                    }
                }

                // Enter or C crafts selected recipe
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::C) {
                    if let Some(recipe) = recipes_in_category.get(state.ui_state.crafting_selected_recipe) {
                        log::info!("Crafting: {}", recipe.id);
                        commands.push(InputCommand::Craft { recipe_id: recipe.id.clone() });
                    }
                }
            } else if state.ui_state.shop_main_tab == 1 {
                // Shop tab - side-by-side Buy/Sell layout
                // Mouse wheel scrolling based on which scroll area the mouse is hovering over
                let (_wheel_x, wheel_y) = mouse_wheel();
                if wheel_y != 0.0 {
                    const SCROLL_SPEED: f32 = 30.0;
                    let item_height = 48.0 + 4.0; // height + spacing

                    // Check which area is being hovered
                    match &state.ui_state.hovered_element {
                        Some(UiElementId::ShopBuyScrollArea) | Some(UiElementId::ShopBuyItem(_)) => {
                            if let Some(ref shop_data) = state.ui_state.shop_data {
                                let max_scroll = ((shop_data.stock.len() as f32) * item_height - 200.0).max(0.0);
                                state.ui_state.shop_buy_scroll = (state.ui_state.shop_buy_scroll - wheel_y * SCROLL_SPEED).clamp(0.0, max_scroll);
                            }
                        }
                        Some(UiElementId::ShopSellScrollArea) | Some(UiElementId::ShopSellItem(_)) => {
                            let inventory_count = state.inventory.slots.iter().filter(|s| s.is_some()).count();
                            let max_scroll = ((inventory_count as f32) * item_height - 200.0).max(0.0);
                            state.ui_state.shop_sell_scroll = (state.ui_state.shop_sell_scroll - wheel_y * SCROLL_SPEED).clamp(0.0, max_scroll);
                        }
                        _ => {}
                    }
                }

                // Keyboard controls for shop
                use crate::game::ShopSubTab;

                // Left/Right or A/D to switch between Buy and Sell panels
                if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                    state.ui_state.shop_sub_tab = ShopSubTab::Buy;
                }
                if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                    state.ui_state.shop_sub_tab = ShopSubTab::Sell;
                }
                // Tab to toggle between panels
                if is_key_pressed(KeyCode::Tab) {
                    state.ui_state.shop_sub_tab = match state.ui_state.shop_sub_tab {
                        ShopSubTab::Buy => ShopSubTab::Sell,
                        ShopSubTab::Sell => ShopSubTab::Buy,
                    };
                }

                // Up/Down or W/S to navigate items in the active panel
                match state.ui_state.shop_sub_tab {
                    ShopSubTab::Buy => {
                        let item_count = state.ui_state.shop_data.as_ref()
                            .map(|d| d.stock.len())
                            .unwrap_or(0);

                        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                            if state.ui_state.shop_selected_buy_index > 0 {
                                state.ui_state.shop_selected_buy_index -= 1;
                                state.ui_state.shop_buy_quantity = 1;
                            }
                        }
                        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                            if state.ui_state.shop_selected_buy_index < item_count.saturating_sub(1) {
                                state.ui_state.shop_selected_buy_index += 1;
                                state.ui_state.shop_buy_quantity = 1;
                            }
                        }

                        // +/- to adjust quantity
                        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
                            state.ui_state.shop_buy_quantity += 1;
                        }
                        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
                            if state.ui_state.shop_buy_quantity > 1 {
                                state.ui_state.shop_buy_quantity -= 1;
                            }
                        }

                        // Enter to confirm buy
                        if is_key_pressed(KeyCode::Enter) {
                            if let Some(ref shop_data) = state.ui_state.shop_data {
                                if let Some(ref npc_id) = state.ui_state.shop_npc_id {
                                    if let Some(stock_item) = shop_data.stock.get(state.ui_state.shop_selected_buy_index) {
                                        audio.play_sfx("buy");
                                        commands.push(InputCommand::ShopBuy {
                                            npc_id: npc_id.clone(),
                                            item_id: stock_item.item_id.clone(),
                                            quantity: state.ui_state.shop_buy_quantity as u32,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    ShopSubTab::Sell => {
                        let inventory_items: Vec<_> = state.inventory.slots.iter()
                            .filter_map(|slot| slot.as_ref())
                            .collect();
                        let item_count = inventory_items.len();

                        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                            if state.ui_state.shop_selected_sell_index > 0 {
                                state.ui_state.shop_selected_sell_index -= 1;
                                state.ui_state.shop_sell_quantity = 1;
                            }
                        }
                        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                            if state.ui_state.shop_selected_sell_index < item_count.saturating_sub(1) {
                                state.ui_state.shop_selected_sell_index += 1;
                                state.ui_state.shop_sell_quantity = 1;
                            }
                        }

                        // +/- to adjust quantity
                        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
                            state.ui_state.shop_sell_quantity += 1;
                        }
                        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
                            if state.ui_state.shop_sell_quantity > 1 {
                                state.ui_state.shop_sell_quantity -= 1;
                            }
                        }

                        // Enter to confirm sell
                        if is_key_pressed(KeyCode::Enter) {
                            if let Some(ref npc_id) = state.ui_state.shop_npc_id {
                                if let Some(inv_slot) = inventory_items.get(state.ui_state.shop_selected_sell_index) {
                                    commands.push(InputCommand::ShopSell {
                                        npc_id: npc_id.clone(),
                                        item_id: inv_slot.item_id.clone(),
                                        quantity: state.ui_state.shop_sell_quantity as u32,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Don't process other input while crafting is open
            return commands;
        }

        // Handle chat input mode
        if state.ui_state.chat_open {
            // Helper to convert character index to byte index
            let char_to_byte_index = |s: &str, char_idx: usize| -> usize {
                s.char_indices()
                    .nth(char_idx)
                    .map(|(byte_idx, _)| byte_idx)
                    .unwrap_or(s.len())
            };

            // Key repeat timing constants
            const INITIAL_DELAY: f64 = 0.4; // 400ms before repeat starts
            const REPEAT_RATE: f64 = 0.035; // ~28 repeats per second

            let current_time = macroquad::time::get_time();

            // Escape cancels chat
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                state.ui_state.chat_cursor = 0;
                state.ui_state.chat_scroll_offset = 0;
                return commands;
            }

            // Enter sends message
            if is_key_pressed(KeyCode::Enter) {
                let text = state.ui_state.chat_input.trim().to_string();
                if !text.is_empty() {
                    audio.play_sfx("send_message");
                    commands.push(InputCommand::Chat { text });
                }
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                state.ui_state.chat_cursor = 0;
                state.ui_state.chat_scroll_offset = 0;
                return commands;
            }

            let char_count = state.ui_state.chat_input.chars().count();

            // Check if any repeatable key is held
            let repeatable_keys = [KeyCode::Left, KeyCode::Right, KeyCode::Backspace, KeyCode::Delete];
            let any_repeatable_held = repeatable_keys.iter().any(|k| is_key_down(*k));

            // Reset repeat state if no repeatable keys held
            if !any_repeatable_held {
                state.ui_state.chat_key_initial_delay = true;
            }

            // Helper to check if we should fire a key action (initial press or repeat)
            let should_fire = |key: KeyCode, state: &mut GameState, current_time: f64| -> bool {
                if is_key_pressed(key) {
                    // Initial press - always fire and start repeat timer
                    state.ui_state.chat_key_repeat_time = current_time;
                    state.ui_state.chat_key_initial_delay = true;
                    return true;
                } else if is_key_down(key) {
                    // Key held - check repeat timing
                    let delay = if state.ui_state.chat_key_initial_delay { INITIAL_DELAY } else { REPEAT_RATE };
                    if current_time - state.ui_state.chat_key_repeat_time >= delay {
                        state.ui_state.chat_key_repeat_time = current_time;
                        state.ui_state.chat_key_initial_delay = false;
                        return true;
                    }
                }
                false
            };

            // Arrow key navigation (drain char queue after to prevent ghost characters)
            if should_fire(KeyCode::Left, state, current_time) {
                if state.ui_state.chat_cursor > 0 {
                    state.ui_state.chat_cursor -= 1;
                }
                while get_char_pressed().is_some() {}
            }
            if should_fire(KeyCode::Right, state, current_time) {
                let char_count = state.ui_state.chat_input.chars().count();
                if state.ui_state.chat_cursor < char_count {
                    state.ui_state.chat_cursor += 1;
                }
                while get_char_pressed().is_some() {}
            }
            // Home/End for quick navigation (no repeat needed)
            if is_key_pressed(KeyCode::Home) {
                state.ui_state.chat_cursor = 0;
                while get_char_pressed().is_some() {}
            }
            if is_key_pressed(KeyCode::End) {
                state.ui_state.chat_cursor = char_count;
                while get_char_pressed().is_some() {}
            }

            // Backspace removes character before cursor
            if should_fire(KeyCode::Backspace, state, current_time) {
                if state.ui_state.chat_cursor > 0 {
                    let byte_idx = char_to_byte_index(&state.ui_state.chat_input, state.ui_state.chat_cursor - 1);
                    state.ui_state.chat_input.remove(byte_idx);
                    state.ui_state.chat_cursor -= 1;
                }
            }

            // Delete removes character at cursor
            if should_fire(KeyCode::Delete, state, current_time) {
                let char_count = state.ui_state.chat_input.chars().count();
                if state.ui_state.chat_cursor < char_count {
                    let byte_idx = char_to_byte_index(&state.ui_state.chat_input, state.ui_state.chat_cursor);
                    state.ui_state.chat_input.remove(byte_idx);
                }
            }

            // Capture typed characters - insert at cursor position
            while let Some(c) = get_char_pressed() {
                // Filter control characters and non-printable special chars (like arrow key ghosts)
                if c.is_control() || !c.is_ascii_graphic() && !c.is_ascii_whitespace() && !c.is_alphanumeric() {
                    continue;
                }
                // Limit chat message length (by character count)
                if state.ui_state.chat_input.chars().count() < 200 {
                    let byte_idx = char_to_byte_index(&state.ui_state.chat_input, state.ui_state.chat_cursor);
                    state.ui_state.chat_input.insert(byte_idx, c);
                    state.ui_state.chat_cursor += 1;
                }
            }

            return commands;
        }

        // Enter key opens chat
        if is_key_pressed(KeyCode::Enter) {
            state.ui_state.chat_open = true;
            state.ui_state.chat_input.clear();
            state.ui_state.chat_cursor = 0;
            state.ui_state.chat_scroll_offset = 0;
            // Drain any accumulated characters from the queue
            while get_char_pressed().is_some() {}
            return commands;
        }

        // Drain character queue when chat is closed to prevent accumulation
        while get_char_pressed().is_some() {}

        // Read which keys are held
        let up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
        let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
        let right = is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);

        // Cancel auto-path if any movement key is pressed
        if up || down || left || right {
            state.clear_auto_path();
        }

        // Determine new direction - only one direction at a time
        // Priority: keep current direction if still held, otherwise pick new one
        let new_dir = match self.current_dir {
            CardinalDir::Up if up => CardinalDir::Up,
            CardinalDir::Down if down => CardinalDir::Down,
            CardinalDir::Left if left => CardinalDir::Left,
            CardinalDir::Right if right => CardinalDir::Right,
            _ => {
                // Current direction released, pick a new one (priority order)
                if up { CardinalDir::Up }
                else if down { CardinalDir::Down }
                else if left { CardinalDir::Left }
                else if right { CardinalDir::Right }
                else { CardinalDir::None }
            }
        };

        // Detect direction changes for face vs move logic
        let dir_changed = new_dir != self.prev_dir;

        // Handle direction key press/release for face vs move
        if dir_changed {
            if new_dir != CardinalDir::None && self.prev_dir == CardinalDir::None {
                // New direction pressed - record time
                self.dir_press_time = current_time;
                self.move_sent = false;
            } else if new_dir == CardinalDir::None && self.prev_dir != CardinalDir::None {
                // Direction released
                let hold_duration = current_time - self.dir_press_time;
                log::info!("[INPUT] Key released: hold_duration={:.3}s, threshold={:.3}s, move_sent={}",
                    hold_duration, FACE_THRESHOLD, self.move_sent);
                if self.move_sent {
                    // Was moving, now stopped - send stop command
                    commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
                    self.last_dx = 0.0;
                    self.last_dy = 0.0;
                    self.last_send_time = current_time;
                } else {
                    // Never sent a move (quick tap or frame timing edge case) - send Face command
                    // But not if attacking - player must finish attack first
                    let attack_anim = state.get_local_player().map_or(false, |p| {
                        matches!(
                            p.animation.state,
                            AnimationState::Attacking | AnimationState::Casting | AnimationState::ShootingBow
                        )
                    });
                    if !attack_anim {
                        let dir = self.prev_dir.to_direction_u8();
                        log::info!("[INPUT] Sending Face command: direction={} (prev_dir={:?})", dir, self.prev_dir);
                        commands.push(InputCommand::Face { direction: dir });
                        self.last_send_time = current_time;
                    }
                }
            } else if new_dir != CardinalDir::None && self.prev_dir != CardinalDir::None {
                // Direction changed while holding
                if self.move_sent {
                    // Already moving - continue moving in new direction immediately (no threshold wait)
                    // move_sent stays true, don't reset dir_press_time
                } else {
                    // Wasn't moving yet (still in threshold wait) - restart timer for new direction
                    self.dir_press_time = current_time;
                }
            }
        }

        self.prev_dir = new_dir;
        self.current_dir = new_dir;

        // Convert direction to velocity
        let (dx, dy): (f32, f32) = match new_dir {
            CardinalDir::Up => (0.0, -1.0),
            CardinalDir::Down => (0.0, 1.0),
            CardinalDir::Left => (-1.0, 0.0),
            CardinalDir::Right => (1.0, 0.0),
            CardinalDir::None => (0.0, 0.0),
        };

        // Only send Move commands if held past the threshold
        // Don't move while attacking - check both Space key and animation state
        let is_attacking = is_key_down(KeyCode::Space) || state.get_local_player().map_or(false, |p| {
            matches!(
                p.animation.state,
                AnimationState::Attacking | AnimationState::Casting | AnimationState::ShootingBow
            )
        });
        if new_dir != CardinalDir::None && !is_attacking {
            let hold_duration = current_time - self.dir_press_time;
            if hold_duration >= FACE_THRESHOLD {
                // Past threshold - check if target tile is walkable before sending movement
                let can_move = if let Some(player) = state.get_local_player() {
                    let player_x = player.x.round() as i32;
                    let player_y = player.y.round() as i32;
                    let target_x = player_x + dx as i32;
                    let target_y = player_y + dy as i32;

                    // Check static tile collision
                    let tile_walkable = state.chunk_manager.is_walkable(target_x as f32, target_y as f32);

                    // Check entity collision
                    let occupied = build_occupied_set(state);
                    let not_occupied = !occupied.contains(&(target_x, target_y));

                    tile_walkable && not_occupied
                } else {
                    false
                };

                let direction_changed = (dx - self.last_dx).abs() > 0.01 || (dy - self.last_dy).abs() > 0.01;
                let time_elapsed = current_time - self.last_send_time >= self.send_interval;
                let should_send = direction_changed || time_elapsed;

                if can_move {
                    if should_send {
                        commands.push(InputCommand::Move { dx, dy });
                        self.last_dx = dx;
                        self.last_dy = dy;
                        self.last_send_time = current_time;
                        self.move_sent = true;
                    }
                } else {
                    // Can't move - face that direction instead
                    if self.move_sent {
                        // Was moving, send stop
                        commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
                        self.move_sent = false;
                    }
                    if should_send {
                        commands.push(InputCommand::Face { direction: new_dir.to_direction_u8() });
                        self.last_dx = dx;
                        self.last_dy = dy;
                        self.last_send_time = current_time;
                    }
                }
            }
        }

        // Path following - generate movement commands when auto-pathing
        // Only follow path if not manually moving and not attacking
        if dx == 0.0 && dy == 0.0 && !is_attacking {
            // Get player position first to avoid borrow conflicts
            let player_pos = state.get_local_player().map(|p| (p.x.round() as i32, p.y.round() as i32));

            // Check if next waypoint is blocked by an entity - if so, cancel path
            let mut path_blocked = false;
            if let (Some((player_x, player_y)), Some(ref path_state)) = (player_pos, &state.auto_path) {
                if path_state.current_index < path_state.path.len() {
                    let (next_x, next_y) = path_state.path[path_state.current_index];

                    // Check if we need to move to reach the waypoint
                    if player_x != next_x || player_y != next_y {
                        let occupied = build_occupied_set(state);
                        if occupied.contains(&(next_x, next_y)) {
                            path_blocked = true;
                        }
                    }
                }
            }

            // If path is blocked by entity, cancel it and stop
            if path_blocked {
                state.auto_path = None;
                commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
                return commands;
            }

            if let (Some((player_x, player_y)), Some(ref mut path_state)) = (player_pos, &mut state.auto_path) {
                // Check if we've reached the current waypoint
                if path_state.current_index < path_state.path.len() {
                    let (target_x, target_y) = path_state.path[path_state.current_index];

                    if player_x == target_x && player_y == target_y {
                        // Move to next waypoint
                        path_state.current_index += 1;
                    }

                    // Generate movement toward current waypoint
                    if path_state.current_index < path_state.path.len() {
                        let (next_x, next_y) = path_state.path[path_state.current_index];
                        let move_dx = (next_x - player_x).signum() as f32;
                        let move_dy = (next_y - player_y).signum() as f32;

                        // Only move in one direction at a time (grid-based movement)
                        if move_dx != 0.0 {
                            commands.push(InputCommand::Move { dx: move_dx, dy: 0.0 });
                        } else if move_dy != 0.0 {
                            commands.push(InputCommand::Move { dx: 0.0, dy: move_dy });
                        }
                    }
                }
            }

            // Check if path completed and handle pickup/interact if needed
            if state.auto_path.as_ref().map(|p| p.current_index >= p.path.len()).unwrap_or(false) {
                // Path completed - check for pickup target
                if let Some(ref path_state) = state.auto_path {
                    if let Some(ref item_id) = path_state.pickup_target {
                        commands.push(InputCommand::Pickup { item_id: item_id.clone() });
                    }
                    // Handle interact target (NPC)
                    if let Some(ref npc_id) = path_state.interact_target {
                        if state.npcs.get(npc_id).map(|n| n.is_alive()).unwrap_or(false) {
                            commands.push(InputCommand::Interact { npc_id: npc_id.clone() });
                        }
                    }
                }
                state.auto_path = None;

                // Send stop command so we don't keep moving in the last direction
                commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
            }
        }

        // Attack (Space key) - holding space continues attacking with cooldown
        // Also stop movement when attacking (player must stand still)
        if is_key_down(KeyCode::Space) {
            // Send stop command if we were moving via keyboard or auto-path
            let was_pathing = state.auto_path.is_some();
            if self.last_dx != 0.0 || self.last_dy != 0.0 || was_pathing {
                commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
                self.last_dx = 0.0;
                self.last_dy = 0.0;
            }
            // Cancel auto-path when attacking
            state.clear_auto_path();

            if current_time - self.last_attack_time >= self.attack_cooldown {
                log::info!("Space held - sending Attack command");
                commands.push(InputCommand::Attack);
                self.last_attack_time = current_time;

                // Set attack animation based on weapon type
                // First, determine the animation type by reading weapon info
                let anim_state = if let Some(player) = state.get_local_player() {
                    if let Some(ref weapon_id) = player.equipped_weapon {
                        if let Some(item_def) = state.item_registry.get(weapon_id) {
                            if item_def.weapon_type.as_deref() == Some("ranged") {
                                AnimationState::ShootingBow
                            } else {
                                AnimationState::Attacking
                            }
                        } else {
                            AnimationState::Attacking
                        }
                    } else {
                        AnimationState::Attacking
                    }
                } else {
                    AnimationState::Attacking
                };

                // Now apply the animation to the player
                if let Some(local_id) = &state.local_player_id.clone() {
                    if let Some(player) = state.players.get_mut(local_id) {
                        player.animation.set_state(anim_state);
                    }
                }
            }
        }

        // Handle mouse clicks on quick slots and inventory (always visible when open)
        if let Some(ref element) = clicked_element {
            match element {
                UiElementId::QuickSlot(idx) => {
                    if mouse_clicked {
                        commands.push(InputCommand::UseItem { slot_index: *idx as u8 });
                    } else if mouse_right_clicked {
                        // Right-click on quick slot opens context menu (if item exists)
                        if state.inventory.slots.get(*idx).and_then(|s| s.as_ref()).is_some() {
                            state.ui_state.context_menu = Some(ContextMenu {
                                target: ContextMenuTarget::InventorySlot(*idx),
                                x: mx,
                                y: my,
                            });
                        }
                    }
                    return commands;
                }
                UiElementId::InventorySlot(idx) => {
                    if mouse_right_clicked {
                        // Right-click opens context menu (if item exists)
                        if state.inventory.slots.get(*idx).and_then(|s| s.as_ref()).is_some() {
                            state.ui_state.context_menu = Some(ContextMenu {
                                target: ContextMenuTarget::InventorySlot(*idx),
                                x: mx,
                                y: my,
                            });
                        }
                    }
                    return commands;
                }
                UiElementId::EquipmentSlot(slot_type) => {
                    if mouse_right_clicked {
                        // Right-click on equipment slot opens context menu (if something is equipped)
                        let has_item = match slot_type.as_str() {
                            "head" => state.get_local_player().and_then(|p| p.equipped_head.as_ref()).is_some(),
                            "body" => state.get_local_player().and_then(|p| p.equipped_body.as_ref()).is_some(),
                            "weapon" => state.get_local_player().and_then(|p| p.equipped_weapon.as_ref()).is_some(),
                            "back" => state.get_local_player().and_then(|p| p.equipped_back.as_ref()).is_some(),
                            "feet" => state.get_local_player().and_then(|p| p.equipped_feet.as_ref()).is_some(),
                            "ring" => state.get_local_player().and_then(|p| p.equipped_ring.as_ref()).is_some(),
                            "gloves" => state.get_local_player().and_then(|p| p.equipped_gloves.as_ref()).is_some(),
                            "necklace" => state.get_local_player().and_then(|p| p.equipped_necklace.as_ref()).is_some(),
                            "belt" => state.get_local_player().and_then(|p| p.equipped_belt.as_ref()).is_some(),
                            _ => false,
                        };
                        if has_item {
                            state.ui_state.context_menu = Some(ContextMenu {
                                target: ContextMenuTarget::EquipmentSlot(slot_type.clone()),
                                x: mx,
                                y: my,
                            });
                        }
                    }
                    return commands;
                }
                UiElementId::GoldDisplay => {
                    if mouse_right_clicked && state.inventory.gold > 0 {
                        // Right-click on gold display opens context menu
                        state.ui_state.context_menu = Some(ContextMenu {
                            target: ContextMenuTarget::Gold,
                            x: mx,
                            y: my,
                        });
                    }
                    return commands;
                }
                UiElementId::GroundItem(item_id) => {
                    if mouse_clicked {
                        // Left-click on ground item - attempt pickup if within range, or path to it
                        if let Some(local_id) = &state.local_player_id {
                            if let Some(player) = state.players.get(local_id) {
                                if let Some(ground_item) = state.ground_items.get(item_id) {
                                    let dx = ground_item.x - player.x;
                                    let dy = ground_item.y - player.y;
                                    let dist = (dx * dx + dy * dy).sqrt();

                                    const PICKUP_RANGE: f32 = 2.0;
                                    if dist < PICKUP_RANGE {
                                        commands.push(InputCommand::Pickup { item_id: item_id.clone() });
                                    } else {
                                        // Out of range - path to an adjacent tile
                                        let player_x = player.x.round() as i32;
                                        let player_y = player.y.round() as i32;
                                        let item_x = ground_item.x.round() as i32;
                                        let item_y = ground_item.y.round() as i32;

                                        // Build occupied set (other players + NPCs)
                                        let occupied = build_occupied_set(state);

                                        const MAX_PATH_DISTANCE: i32 = 32;
                                        if let Some((dest, path)) = pathfinding::find_path_to_adjacent(
                                            (player_x, player_y),
                                            (item_x, item_y),
                                            &state.chunk_manager,
                                            &occupied,
                                            MAX_PATH_DISTANCE,
                                        ) {
                                            state.auto_path = Some(PathState {
                                                path,
                                                current_index: 0,
                                                destination: dest,
                                                pickup_target: Some(item_id.clone()),
                                                interact_target: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return commands;
                }
                _ => {}
            }
        }

        // Target selection (left click) - only if not clicking on UI
        if mouse_clicked && clicked_element.is_none() {
            let (mouse_x, mouse_y) = mouse_position();
            let (world_x, world_y) = screen_to_world(mouse_x, mouse_y, &state.camera);

            // Get the clicked tile coordinates
            let clicked_tile_x = world_x.round() as i32;
            let clicked_tile_y = world_y.round() as i32;

            // Find entity on the exact clicked tile
            let mut clicked_player: Option<String> = None;
            let mut clicked_npc: Option<String> = None;

            // Check players - must be on the exact clicked tile
            for (id, player) in &state.players {
                // Don't allow targeting self
                if state.local_player_id.as_ref() == Some(id) {
                    continue;
                }

                let player_tile_x = player.x.round() as i32;
                let player_tile_y = player.y.round() as i32;

                if player_tile_x == clicked_tile_x && player_tile_y == clicked_tile_y {
                    clicked_player = Some(id.clone());
                    break;
                }
            }

            // Check NPCs - must be on the exact clicked tile
            for (id, npc) in &state.npcs {
                // Only allow interacting with alive NPCs
                if !npc.is_alive() {
                    continue;
                }

                let npc_tile_x = npc.x.round() as i32;
                let npc_tile_y = npc.y.round() as i32;

                if npc_tile_x == clicked_tile_x && npc_tile_y == clicked_tile_y {
                    clicked_npc = Some(id.clone());
                    break;
                }
            }

            // Prioritize NPC interaction over player targeting
            if let Some(npc_id) = clicked_npc {
                // Check if NPC is hostile (monster) or friendly (shop/quest)
                let is_hostile = state.npcs.get(&npc_id).map(|n| n.is_hostile()).unwrap_or(true);

                if is_hostile {
                    // Hostile NPC (monster) - target them for combat
                    commands.push(InputCommand::Target { entity_id: npc_id });
                } else {
                    // Friendly NPC - interact or pathfind-to-interact
                    const INTERACT_RANGE: f32 = 2.5;
                    if let Some(local_id) = &state.local_player_id {
                        if let Some(player) = state.players.get(local_id) {
                            if let Some(npc) = state.npcs.get(&npc_id) {
                                let dx = npc.x - player.x;
                                let dy = npc.y - player.y;
                                let dist_to_player = (dx * dx + dy * dy).sqrt();

                                if dist_to_player < INTERACT_RANGE {
                                    // Within range - immediate interact
                                    commands.push(InputCommand::Interact { npc_id });
                                } else {
                                    // Out of range - pathfind to adjacent tile
                                    let player_x = player.x.round() as i32;
                                    let player_y = player.y.round() as i32;
                                    let npc_x = npc.x.round() as i32;
                                    let npc_y = npc.y.round() as i32;

                                    // Build occupied set (other players + NPCs)
                                    let occupied = build_occupied_set(state);

                                    const MAX_PATH_DISTANCE: i32 = 32;
                                    if let Some((dest, path)) = pathfinding::find_path_to_adjacent(
                                        (player_x, player_y),
                                        (npc_x, npc_y),
                                        &state.chunk_manager,
                                        &occupied,
                                        MAX_PATH_DISTANCE,
                                    ) {
                                        state.auto_path = Some(PathState {
                                            path,
                                            current_index: 0,
                                            destination: dest,
                                            pickup_target: None,
                                            interact_target: Some(npc_id),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(entity_id) = clicked_player {
                // Player clicked - target them
                commands.push(InputCommand::Target { entity_id });
            } else {
                // Clicked on empty space - try to path there
                let tile_x = world_x.round() as i32;
                let tile_y = world_y.round() as i32;

                // Only path if within range and walkable
                const MAX_PATH_DISTANCE: i32 = 32;

                if let Some(player) = state.get_local_player() {
                    let player_x = player.x.round() as i32;
                    let player_y = player.y.round() as i32;
                    let dist = (tile_x - player_x).abs().max((tile_y - player_y).abs());

                    if dist <= MAX_PATH_DISTANCE && state.chunk_manager.is_walkable(tile_x as f32, tile_y as f32) {
                        // Build occupied set (other players + NPCs)
                        let occupied = build_occupied_set(state);

                        // Calculate path using A*
                        if let Some(path) = pathfinding::find_path(
                            (player_x, player_y),
                            (tile_x, tile_y),
                            &state.chunk_manager,
                            &occupied,
                            MAX_PATH_DISTANCE,
                        ) {
                            state.auto_path = Some(PathState {
                                path,
                                current_index: 0,
                                destination: (tile_x, tile_y),
                                pickup_target: None,
                                interact_target: None,
                            });
                        }
                    }
                }

                // Also clear target when clicking empty space
                if state.selected_entity_id.is_some() {
                    commands.push(InputCommand::ClearTarget);
                }
            }
        }

        // Escape key - close any open panel first, then clear target, then open escape menu
        if is_key_pressed(KeyCode::Escape) {
            // Check if any panel is open and close it
            if state.ui_state.inventory_open || state.ui_state.character_panel_open
                || state.ui_state.character_open || state.ui_state.social_open
                || state.ui_state.skills_open {
                audio.play_sfx("enter");
                state.ui_state.inventory_open = false;
                state.ui_state.character_panel_open = false;
                state.ui_state.character_open = false;
                state.ui_state.social_open = false;
                state.ui_state.skills_open = false;
            } else if state.selected_entity_id.is_some() {
                commands.push(InputCommand::ClearTarget);
            } else {
                // No target selected and no panels open - open escape menu
                audio.play_sfx("enter");
                state.ui_state.escape_menu_open = true;
            }
        }

        // Toggle inventory (I key) with mutual exclusivity
        if is_key_pressed(KeyCode::I) {
            audio.play_sfx("enter");
            if state.ui_state.inventory_open {
                state.ui_state.inventory_open = false;
            } else {
                state.ui_state.inventory_open = true;
                state.ui_state.character_panel_open = false;
                state.ui_state.character_open = false;
                state.ui_state.social_open = false;
                state.ui_state.skills_open = false;
            }
        }

        // Toggle character panel (C key) with mutual exclusivity
        if is_key_pressed(KeyCode::C) {
            audio.play_sfx("enter");
            if state.ui_state.character_panel_open {
                state.ui_state.character_panel_open = false;
            } else {
                state.ui_state.character_panel_open = true;
                state.ui_state.inventory_open = false;
                state.ui_state.character_open = false;
                state.ui_state.social_open = false;
                state.ui_state.skills_open = false;
            }
        }

        // Use/equip items (1-5 keys for quick slots)
        let quick_slot_keys = [
            (KeyCode::Key1, 0usize),
            (KeyCode::Key2, 1usize),
            (KeyCode::Key3, 2usize),
            (KeyCode::Key4, 3usize),
            (KeyCode::Key5, 4usize),
        ];
        for (key, slot_idx) in quick_slot_keys {
            if is_key_pressed(key) {
                if let Some(Some(slot)) = state.inventory.slots.get(slot_idx) {
                    let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                    if item_def.equipment.is_some() {
                        // Equippable item - equip it
                        commands.push(InputCommand::Equip { slot_index: slot_idx as u8 });
                    } else {
                        // Not equippable - use it (e.g., consume potion)
                        commands.push(InputCommand::UseItem { slot_index: slot_idx as u8 });
                    }
                }
            }
        }

        // Pickup nearest item (F key)
        if is_key_pressed(KeyCode::F) {
            // Get local player position
            if let Some(local_id) = &state.local_player_id {
                if let Some(player) = state.players.get(local_id) {
                    // Find nearest item within pickup range (2 tiles)
                    const PICKUP_RANGE: f32 = 2.0;
                    let mut nearest_item: Option<(String, f32)> = None;

                    for (id, item) in &state.ground_items {
                        let dx = item.x - player.x;
                        let dy = item.y - player.y;
                        let dist = (dx * dx + dy * dy).sqrt();

                        if dist < PICKUP_RANGE {
                            if nearest_item.is_none() || dist < nearest_item.as_ref().unwrap().1 {
                                nearest_item = Some((id.clone(), dist));
                            }
                        }
                    }

                    if let Some((item_id, _)) = nearest_item {
                        commands.push(InputCommand::Pickup { item_id });
                    }
                }
            }
        }

        // Interact with nearest NPC (E key)
        if is_key_pressed(KeyCode::E) {
            if let Some(local_id) = &state.local_player_id {
                if let Some(player) = state.players.get(local_id) {
                    // Find nearest NPC within interaction range (2.5 tiles)
                    const INTERACT_RANGE: f32 = 2.5;
                    let mut nearest_npc: Option<(String, f32)> = None;

                    for (id, npc) in &state.npcs {
                        // Only interact with alive NPCs
                        if !npc.is_alive() {
                            continue;
                        }

                        let dx = npc.x - player.x;
                        let dy = npc.y - player.y;
                        let dist = (dx * dx + dy * dy).sqrt();

                        if dist < INTERACT_RANGE {
                            if nearest_npc.is_none() || dist < nearest_npc.as_ref().unwrap().1 {
                                nearest_npc = Some((id.clone(), dist));
                            }
                        }
                    }

                    if let Some((npc_id, _)) = nearest_npc {
                        log::info!("Interacting with NPC: {}", npc_id);
                        commands.push(InputCommand::Interact { npc_id });
                    }
                }
            }
        }

        // Toggle quest log (Q key)
        if is_key_pressed(KeyCode::Q) {
            state.ui_state.quest_log_open = !state.ui_state.quest_log_open;
        }

        commands
    }

    /// Get current movement direction (for client-side prediction)
    pub fn get_movement(&self) -> (f32, f32) {
        (self.last_dx, self.last_dy)
    }
}
