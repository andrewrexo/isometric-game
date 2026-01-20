# Chat Channels & System Logging Design

## Overview

Add a chat channel system to support system logging of game events (XP gains, level ups, quest completions, shop transactions) alongside player chat.

## Data Model

### ChatChannel Enum

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatChannel {
    Local,      // Nearby players only (current default)
    Global,     // Server-wide player chat
    System,     // XP gains, quest completions, shop transactions
    // Future:
    // Party,
    // Guild,
}
```

### Updated ChatMessage Struct

```rust
pub struct ChatMessage {
    pub sender_name: String,  // Player name or "[System]"
    pub text: String,
    pub timestamp: f64,
    pub channel: ChatChannel,
}

impl ChatMessage {
    pub fn player(sender_name: String, text: String) -> Self {
        ChatMessage {
            sender_name,
            text,
            timestamp: macroquad::time::get_time(),
            channel: ChatChannel::Local,
        }
    }

    pub fn system(text: String) -> Self {
        ChatMessage {
            sender_name: "[System]".to_string(),
            text,
            timestamp: macroquad::time::get_time(),
            channel: ChatChannel::System,
        }
    }
}
```

## Rendering

Channel-specific colors and formatting:

| Channel | Color | Format |
|---------|-------|--------|
| Local | White | `PlayerName: message` |
| Global | Sky Blue | `[G] PlayerName: message` |
| System | Yellow | `[System] message` |

## System Events to Log

1. **XP Gained** (local player only): `+25 Combat XP`
2. **Level Up** (local player only): `Combat leveled up to 5!`
3. **Quest Complete**: `Quest 'Goblin Slayer' complete!`
4. **Shop Buy**: `Bought 5x Iron Ore for 50g`
5. **Shop Sell**: `Sold 3x Copper Ore for 15g`

## Files to Modify

1. `client/src/game/state.rs` - Add `ChatChannel` enum, update `ChatMessage` struct
2. `client/src/render/renderer.rs` - Update chat rendering with channel colors
3. `client/src/network/client.rs` - Add system messages in event handlers:
   - `skillXp` handler
   - `skillLevelUp` handler
   - `questCompleted` handler
   - `shopResult` handler
   - `chatMessage` handler (update to use new constructor)
