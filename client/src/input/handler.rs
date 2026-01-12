use macroquad::prelude::*;
use crate::game::{GameState, ContextMenu, DragState, DragSource, PathState, pathfinding};
use crate::render::isometric::screen_to_world;
use crate::ui::{UiElementId, UiLayout};

/// Input commands that can be sent to the server
#[derive(Debug, Clone)]
pub enum InputCommand {
    Move { dx: f32, dy: f32 },
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
    DropItem { slot_index: u8, quantity: u32 },
    SwapSlots { from_slot: u8, to_slot: u8 },
}

/// Cardinal directions for isometric movement (no diagonals)
#[derive(Clone, Copy, PartialEq, Eq)]
enum CardinalDir {
    None,
    Up,
    Down,
    Left,
    Right,
}

pub struct InputHandler {
    // Track last sent velocity to detect changes
    last_dx: f32,
    last_dy: f32,
    // Track which direction was pressed first (for priority)
    current_dir: CardinalDir,
    // Send commands at server tick rate
    last_send_time: f64,
    send_interval: f64,
    // Attack cooldown tracking (matches server cooldown)
    last_attack_time: f64,
    attack_cooldown: f64,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            last_dx: 0.0,
            last_dy: 0.0,
            current_dir: CardinalDir::None,
            last_send_time: 0.0,
            send_interval: 0.05, // 50ms = 20Hz (matches server tick rate)
            last_attack_time: 0.0,
            attack_cooldown: 0.8, // 800 ms (matches server ATTACK_COOLDOWN_MS)
        }
    }

    pub fn process(&mut self, state: &mut GameState, layout: &UiLayout) -> Vec<InputCommand> {
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
        } else {
            state.hovered_tile = None;
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
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }

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
                                        // Check level requirement
                                        let level_ok = state.get_local_player()
                                            .map(|p| p.level >= equip.level_required)
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
                                                    _ => {}
                                                }
                                            }
                                        }
                                        state.inventory.clear_slot(*from_idx);

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
                            // Dropped on non-inventory element, cancel drag
                        }
                    }
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
                            _ => None,
                        };
                        if let Some(item_id) = equipped_item {
                            // Start drag from equipment slot
                            state.ui_state.drag_state = Some(DragState {
                                source: DragSource::Equipment(slot_type.clone()),
                                item_id,
                                quantity: 1, // Equipment is always quantity 1
                            });
                            return commands;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Handle context menu interactions first
        if state.ui_state.context_menu.is_some() {
            if let Some(ref element) = clicked_element {
                if mouse_clicked {
                    match element {
                        UiElementId::ContextMenuOption(option_idx) => {
                            // Get menu info before clearing it
                            let menu = state.ui_state.context_menu.take().unwrap();

                            if menu.is_equipment {
                                // Equipment slot context menu - only unequip option
                                if *option_idx == 0 {
                                    if let Some(ref slot_type) = menu.equipment_slot {
                                        commands.push(InputCommand::Unequip {
                                            slot_type: slot_type.clone(),
                                            target_slot: None, // Use first available slot
                                        });
                                    }
                                }
                            } else {
                                // Inventory slot context menu
                                // Check if item is equippable to determine option indices
                                let is_equippable = state.inventory.slots.get(menu.slot_index)
                                    .and_then(|s| s.as_ref())
                                    .map(|slot| {
                                        let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                                        item_def.equipment.is_some()
                                    })
                                    .unwrap_or(false);

                                if is_equippable {
                                    // Options: Equip (0), Drop (1)
                                    match option_idx {
                                        0 => commands.push(InputCommand::Equip { slot_index: menu.slot_index as u8 }),
                                        1 => {
                                            if let Some(slot) = state.inventory.slots.get(menu.slot_index).and_then(|s| s.as_ref()) {
                                                commands.push(InputCommand::DropItem { slot_index: menu.slot_index as u8, quantity: slot.quantity as u32 });
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Options: Drop (0) only
                                    if *option_idx == 0 {
                                        if let Some(slot) = state.inventory.slots.get(menu.slot_index).and_then(|s| s.as_ref()) {
                                            commands.push(InputCommand::DropItem { slot_index: menu.slot_index as u8, quantity: slot.quantity as u32 });
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

        // Handle escape menu
        if state.ui_state.escape_menu_open {
            // Handle mouse clicks on escape menu elements
            if let Some(ref element) = clicked_element {
                if mouse_clicked {
                    match element {
                        UiElementId::EscapeMenuZoom1x => {
                            state.camera.zoom = 1.0;
                            state.ui_state.escape_menu_open = false;
                            return commands;
                        }
                        UiElementId::EscapeMenuZoom2x => {
                            state.camera.zoom = 2.0;
                            state.ui_state.escape_menu_open = false;
                            return commands;
                        }
                        UiElementId::EscapeMenuDisconnect => {
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
            // Handle mouse clicks on crafting elements
            if let Some(ref element) = clicked_element {
                match element {
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
                    _ => {}
                }
            }

            // Escape or E closes crafting
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::E) {
                state.ui_state.crafting_open = false;
                state.ui_state.crafting_npc_id = None;
                return commands;
            }

            // Get unique categories from recipes
            let categories: Vec<&str> = {
                let mut cats: Vec<&str> = state.recipe_definitions.iter()
                    .map(|r| r.category.as_str())
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
            let current_category = categories.get(state.ui_state.crafting_selected_category).copied().unwrap_or("consumables");
            let recipes_in_category: Vec<&crate::game::RecipeDefinition> = state.recipe_definitions.iter()
                .filter(|r| r.category == current_category)
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

            // Don't process other input while crafting is open
            return commands;
        }

        // Handle chat input mode
        if state.ui_state.chat_open {
            // Escape cancels chat
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                return commands;
            }

            // Enter sends message
            if is_key_pressed(KeyCode::Enter) {
                let text = state.ui_state.chat_input.trim().to_string();
                if !text.is_empty() {
                    commands.push(InputCommand::Chat { text });
                }
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                return commands;
            }

            // Backspace removes last character
            if is_key_pressed(KeyCode::Backspace) {
                state.ui_state.chat_input.pop();
            }

            // Capture typed characters
            while let Some(c) = get_char_pressed() {
                // Filter control characters
                if c.is_control() {
                    continue;
                }
                // Limit chat message length
                if state.ui_state.chat_input.len() < 200 {
                    state.ui_state.chat_input.push(c);
                }
            }

            return commands;
        }

        // Enter key opens chat
        if is_key_pressed(KeyCode::Enter) {
            state.ui_state.chat_open = true;
            state.ui_state.chat_input.clear();
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

        self.current_dir = new_dir;

        // Convert direction to velocity
        let (dx, dy): (f32, f32) = match new_dir {
            CardinalDir::Up => (0.0, -1.0),
            CardinalDir::Down => (0.0, 1.0),
            CardinalDir::Left => (-1.0, 0.0),
            CardinalDir::Right => (1.0, 0.0),
            CardinalDir::None => (0.0, 0.0),
        };

        // Check if movement direction changed
        let direction_changed = (dx - self.last_dx).abs() > 0.01 || (dy - self.last_dy).abs() > 0.01;
        let time_elapsed = current_time - self.last_send_time >= self.send_interval;

        // Send command if: direction changed OR (moving AND enough time passed)
        let should_send = direction_changed || (time_elapsed && (dx != 0.0 || dy != 0.0));

        if should_send {
            commands.push(InputCommand::Move { dx, dy });
            self.last_dx = dx;
            self.last_dy = dy;
            self.last_send_time = current_time;
        }

        // Path following - generate movement commands when auto-pathing
        // Only follow path if not manually moving
        if dx == 0.0 && dy == 0.0 {
            // Get player position first to avoid borrow conflicts
            let player_pos = state.get_local_player().map(|p| (p.x.round() as i32, p.y.round() as i32));

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

            // Check if path completed and handle pickup if needed
            if state.auto_path.as_ref().map(|p| p.current_index >= p.path.len()).unwrap_or(false) {
                // Path completed - check for pickup target
                if let Some(ref path_state) = state.auto_path {
                    if let Some(ref item_id) = path_state.pickup_target {
                        commands.push(InputCommand::Pickup { item_id: item_id.clone() });
                    }
                }
                state.auto_path = None;

                // Send stop command so we don't keep moving in the last direction
                commands.push(InputCommand::Move { dx: 0.0, dy: 0.0 });
            }
        }

        // Attack (Space key) - holding space continues attacking with cooldown
        if is_key_down(KeyCode::Space) {
            if current_time - self.last_attack_time >= self.attack_cooldown {
                log::info!("Space held - sending Attack command");
                commands.push(InputCommand::Attack);
                self.last_attack_time = current_time;
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
                                slot_index: *idx,
                                x: mx,
                                y: my,
                                is_equipment: false,
                                equipment_slot: None,
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
                                slot_index: *idx,
                                x: mx,
                                y: my,
                                is_equipment: false,
                                equipment_slot: None,
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
                            _ => false,
                        };
                        if has_item {
                            state.ui_state.context_menu = Some(ContextMenu {
                                slot_index: 0, // Not used for equipment
                                x: mx,
                                y: my,
                                is_equipment: true,
                                equipment_slot: Some(slot_type.clone()),
                            });
                        }
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

                                        const MAX_PATH_DISTANCE: i32 = 32;
                                        if let Some((dest, path)) = pathfinding::find_path_to_adjacent(
                                            (player_x, player_y),
                                            (item_x, item_y),
                                            &state.chunk_manager,
                                            MAX_PATH_DISTANCE,
                                        ) {
                                            state.auto_path = Some(PathState {
                                                path,
                                                current_index: 0,
                                                destination: dest,
                                                pickup_target: Some(item_id.clone()),
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

            // Find nearest entity within click radius (1.5 tiles)
            const CLICK_RADIUS: f32 = 1.5;
            let mut nearest_entity: Option<(String, f32)> = None;

            // Check players
            for (id, player) in &state.players {
                // Don't allow targeting self
                if state.local_player_id.as_ref() == Some(id) {
                    continue;
                }

                let dx: f32 = player.x - world_x;
                let dy: f32 = player.y - world_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < CLICK_RADIUS {
                    if nearest_entity.is_none() || dist < nearest_entity.as_ref().unwrap().1 {
                        nearest_entity = Some((id.clone(), dist));
                    }
                }
            }

            // Check NPCs (prioritize over players for targeting)
            for (id, npc) in &state.npcs {
                // Only allow targeting alive NPCs
                if !npc.is_alive() {
                    continue;
                }

                let dx: f32 = npc.x - world_x;
                let dy: f32 = npc.y - world_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < CLICK_RADIUS {
                    if nearest_entity.is_none() || dist < nearest_entity.as_ref().unwrap().1 {
                        nearest_entity = Some((id.clone(), dist));
                    }
                }
            }

            if let Some((entity_id, _)) = nearest_entity {
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
                        // Calculate path using A*
                        if let Some(path) = pathfinding::find_path(
                            (player_x, player_y),
                            (tile_x, tile_y),
                            &state.chunk_manager,
                            MAX_PATH_DISTANCE,
                        ) {
                            state.auto_path = Some(PathState {
                                path,
                                current_index: 0,
                                destination: (tile_x, tile_y),
                                pickup_target: None,
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

        // Escape key - clear target first, then open escape menu if no target
        if is_key_pressed(KeyCode::Escape) {
            if state.selected_entity_id.is_some() {
                commands.push(InputCommand::ClearTarget);
            } else {
                // No target selected - open escape menu
                state.ui_state.escape_menu_open = true;
            }
        }

        // Toggle inventory (I key)
        if is_key_pressed(KeyCode::I) {
            state.ui_state.inventory_open = !state.ui_state.inventory_open;
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
