# Arena System Design

**Goal:** Create a PvP arena system where players can battle each other in two modes: Party Mode (first to 3 hits) and Death Match (fight to zero HP with item/gold wagers).

**Date:** 2026-01-17

---

## Core Concept

The Arena is a physical building in the game world. Players walk inside to enter the Arena Lobby - a separate instance where they can queue for matches, challenge other waiting players, or spectate ongoing fights.

### Entry Flow

1. Player walks into the Arena building in the world
2. Player is teleported to the Arena Lobby instance
3. In the lobby, player can:
   - Talk to the Arena NPC to join a matchmaking queue (selects mode: Party or Death Match)
   - See other players waiting in the lobby
   - Right-click another player to issue a direct challenge
   - Watch any ongoing match as a spectator
4. When matched (via queue or challenge accepted), both players teleport to the Arena Floor - a simple flat combat space
5. After the match ends, both players return to the lobby

Players can leave the lobby at any time (before a match starts) and return to the world.

---

## Game Modes

### Party Mode (First to 3 Hits)

- Fast, casual matches - first player to land 3 hits wins
- Damage amount doesn't matter, only hit count
- No wagers, no stakes - pure fun
- Normal movement and attack mechanics apply
- Match typically lasts 10-30 seconds
- Great for warming up, practicing, or casual competition

### Death Match (Battle to the Death)

- Fight until one player's HP reaches zero
- Normal combat rules apply (equipment stats, damage/defense bonuses)
- **No consumables** allowed during the fight (no healing potions)
- **Wager required** - both players must agree on stakes before the fight begins
- Higher tension, meaningful consequences

### Wager Negotiation Flow (Death Match only)

1. When match is found/challenge accepted, wager UI opens
2. Challenger proposes their stake (gold amount and/or inventory items)
3. Opponent sees the proposal and can:
   - **Accept** - match begins with those stakes
   - **Counter-offer** - propose different stakes
   - **Decline** - cancel the match, return to lobby
4. Negotiation continues until both accept or someone declines
5. Staked items/gold are held in escrow during the fight
6. Winner receives both stakes

---

## Match Format

- **1v1 only** for MVP
- **Arena map:** Simple flat arena, no obstacles

---

## User Interface

### Lobby UI

- Player list showing everyone currently in the lobby
- Each player entry shows: name, level, current status (idle/queued/fighting)
- Queue buttons: "Join Party Queue" and "Join Death Match Queue"
- "Leave Arena" button to return to the world
- Spectate button when a match is in progress

### Match HUD (during combat)

- Large health bars for both fighters at top of screen (left: you, right: opponent)
- Player names and levels displayed above health bars
- **Party Mode addition:** Hit counter showing score (e.g., "2 - 1")
- Clean, minimal - doesn't obstruct the action

### Wager UI (Death Match pre-fight)

- Modal dialog showing both players
- Your offer panel: gold input field + inventory grid to select items
- Opponent's offer panel: shows what they're staking
- Accept / Counter / Decline buttons
- Timer to prevent stalling (60 seconds to agree or match cancelled)

### Results Screen (post-match)

- Winner announcement with dramatic presentation
- Rewards transferred summary (for Death Match: "You won 500 gold + Iron Sword")
- "Rematch" button (sends challenge to opponent)
- "Return to Lobby" button

---

## Backend Architecture

### New Server Components

**ArenaLobby** (similar to GameRoom)
- Separate instance from the main game world
- Maintains list of players in lobby, their queue status
- Handles matchmaking queue (FIFO, separate queues per mode)
- Broadcasts lobby state updates (player joined/left/matched)

**ArenaMatch** (short-lived combat instance)
- Created when two players are matched
- Holds match state: mode, players, hit counts, wager escrow
- Runs same 50ms tick loop for combat
- Enforces mode-specific rules (hit counting, consumable blocking)
- Handles match end: determine winner, transfer wagers, notify players
- Destroyed after players return to lobby

### New Protocol Messages

**Client → Server:**
- `EnterArena` - request to enter lobby from world
- `LeaveArena` - return to world
- `JoinQueue { mode }` - join matchmaking
- `LeaveQueue` - cancel queue
- `ChallengePlayer { target_id, mode }` - direct challenge
- `RespondChallenge { accept, challenger_id }` - accept/decline
- `ProposeWager { gold, item_slots }` - wager offer
- `AcceptWager` / `DeclineWager` - wager response

**Server → Client:**
- `ArenaLobbyState { players, current_match }` - lobby sync
- `MatchFound { opponent, mode }` - queue pop
- `ChallengeReceived { challenger, mode }` - incoming challenge
- `WagerProposal { gold, items }` - opponent's offer
- `MatchStart { arena_state }` - fight begins
- `HitLanded { attacker, hit_counts }` - party mode hit update
- `MatchEnd { winner, rewards }` - fight over

---

## Spectating

- Players in lobby can watch any ongoing match
- Spectators see the arena floor with both fighters
- Spectators receive same state sync as fighters (positions, HP, hits)
- Spectators use the Match HUD (read-only, no controls)
- When match ends, spectators return to lobby view automatically

---

## Edge Cases & Rules

| Scenario | Behavior |
|----------|----------|
| Player disconnects mid-match | Disconnector forfeits, opponent wins wager |
| Player disconnects in lobby | Removed from queue, challenges cancelled |
| Player disconnects during wager negotiation | Match cancelled, no wager lost |
| Wager negotiation timeout (60s) | Match cancelled, both return to queue/lobby |
| Queue with no opponents | Player waits, can cancel anytime |
| Challenge declined | Challenger notified, both stay in lobby |
| Both players die same tick | Lower HP at moment of death loses (edge case - nearly impossible) |

---

## Future Enhancements (Not in MVP)

- Spectator-specific UI with fighter nameplates
- Cheer/emote system for spectators
- Multiple arena map layouts
- Team modes (2v2, 3v3)
- Free-for-all mode
- Ranking/ladder system
- Match replay system
