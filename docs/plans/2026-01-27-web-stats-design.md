# New Aeven Web Stats Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a standalone web app showing game world stats (dashboard, online players, leaderboards, item registry) powered by new read-only API endpoints on the Rust game server.

**Architecture:** New read-only GET endpoints added to the existing Axum server expose game stats as JSON. A separate Vite + React + TypeScript app (`web-stats/`) consumes them. Dark fantasy theme with professional design.

**Tech Stack:** Vite, React 19, TypeScript, Tailwind CSS v4, React Router, TanStack Query

---

### Task 1: Server — Stats API Endpoints

**Files:**
- Modify: `rust-server/src/main.rs` (add 4 route handlers + wire into router)

**Step 1: Add the stats endpoint handlers**

Add these handlers before the router setup (around line 990, near the `health_check` handler):

```rust
// ============================================================================
// Stats API (read-only, no auth required)
// ============================================================================

#[derive(Serialize)]
struct StatsOverview {
    online_players: usize,
    total_characters: i64,
    total_accounts: i64,
}

async fn stats_overview(State(state): State<AppState>) -> impl IntoResponse {
    // Count online players across all rooms
    let mut online = 0;
    for room in state.rooms.iter() {
        online += room.player_count().await;
    }

    // Query DB for totals
    let total_characters: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters")
        .fetch_one(&state.db.pool())
        .await
        .unwrap_or(0);

    let total_accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
        .fetch_one(&state.db.pool())
        .await
        .unwrap_or(0);

    Json(StatsOverview {
        online_players: online,
        total_characters,
        total_accounts,
    })
}

#[derive(Serialize)]
struct OnlinePlayer {
    name: String,
    combat_level: i32,
    hitpoints_level: i32,
    combat_skill_level: i32,
    total_level: i32,
}

async fn stats_online(State(state): State<AppState>) -> impl IntoResponse {
    let mut players = Vec::new();
    for room in state.rooms.iter() {
        for p in room.get_all_players().await {
            players.push(OnlinePlayer {
                name: p.name.clone(),
                combat_level: p.skills.combat_level(),
                hitpoints_level: p.skills.hitpoints.level,
                combat_skill_level: p.skills.combat.level,
                total_level: p.skills.total_level(),
            });
        }
    }
    Json(players)
}

#[derive(Deserialize)]
struct LeaderboardQuery {
    sort: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct LeaderboardEntry {
    name: String,
    combat_level: i32,
    hitpoints_level: i32,
    combat_skill_level: i32,
    total_level: i32,
    played_time: i64,
}

async fn stats_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);

    // Fetch all characters with skills from DB
    let rows = sqlx::query(
        "SELECT name, skills_json, played_time FROM characters ORDER BY played_time DESC"
    )
    .fetch_all(&state.db.pool())
    .await
    .unwrap_or_default();

    let sort_by = params.sort.as_deref().unwrap_or("combat_level");

    let mut entries: Vec<LeaderboardEntry> = rows.iter().filter_map(|row| {
        let name: String = row.try_get("name").ok()?;
        let skills_json: Option<String> = row.try_get("skills_json").ok()?;
        let played_time: i64 = row.try_get("played_time").unwrap_or(0);

        let skills: Skills = if let Some(ref json) = skills_json {
            serde_json::from_str(json).unwrap_or_default()
        } else {
            Skills::default()
        };

        Some(LeaderboardEntry {
            name,
            combat_level: skills.combat_level(),
            hitpoints_level: skills.hitpoints.level,
            combat_skill_level: skills.combat.level,
            total_level: skills.total_level(),
            played_time,
        })
    }).collect();

    // Sort by requested field
    match sort_by {
        "total_level" => entries.sort_by(|a, b| b.total_level.cmp(&a.total_level)),
        "hitpoints" => entries.sort_by(|a, b| b.hitpoints_level.cmp(&a.hitpoints_level)),
        "combat_skill" => entries.sort_by(|a, b| b.combat_skill_level.cmp(&a.combat_skill_level)),
        _ => entries.sort_by(|a, b| b.combat_level.cmp(&a.combat_level).then(b.total_level.cmp(&a.total_level))),
    }

    entries.truncate(limit as usize);
    Json(entries)
}

#[derive(Serialize)]
struct StatsItem {
    id: String,
    display_name: String,
    sprite: String,
    description: String,
    category: String,
    max_stack: i32,
    base_price: i32,
    sellable: bool,
    equipment: Option<StatsEquipment>,
}

#[derive(Serialize)]
struct StatsEquipment {
    slot_type: String,
    attack_level_required: i32,
    defence_level_required: i32,
    attack_bonus: i32,
    strength_bonus: i32,
    defence_bonus: i32,
    weapon_type: String,
    range: i32,
}

async fn stats_items(State(state): State<AppState>) -> impl IntoResponse {
    use crate::data::item_def::EquipmentSlot;

    let items: Vec<StatsItem> = state.item_registry.all().map(|item| {
        let equipment = item.equipment.as_ref().and_then(|e| {
            if e.slot_type == EquipmentSlot::None { return None; }
            Some(StatsEquipment {
                slot_type: e.slot_type.as_str().to_string(),
                attack_level_required: e.attack_level_required,
                defence_level_required: e.defence_level_required,
                attack_bonus: e.attack_bonus,
                strength_bonus: e.strength_bonus,
                defence_bonus: e.defence_bonus,
                weapon_type: format!("{:?}", e.weapon_type).to_lowercase(),
                range: e.range,
            })
        });

        StatsItem {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            sprite: item.sprite.clone(),
            description: item.description.clone(),
            category: format!("{:?}", item.category).to_lowercase(),
            max_stack: item.max_stack,
            base_price: item.base_price,
            sellable: item.sellable,
            equipment,
        }
    }).collect();

    Json(items)
}
```

**Step 2: Expose db pool and add routes**

In `rust-server/src/db.rs`, add a public accessor for the pool:

```rust
impl Database {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
```

In `main.rs`, add these routes to the router (insert before the `.layer(CorsLayer...)` line):

```rust
// Stats API (public, read-only)
.route("/api/stats/overview", get(stats_overview))
.route("/api/stats/online", get(stats_online))
.route("/api/stats/leaderboard", get(stats_leaderboard))
.route("/api/stats/items", get(stats_items))
```

Also add this import at the top of main.rs:

```rust
use crate::skills::Skills;
```

**Step 3: Verify server compiles**

Run: `cd rust-server && cargo check`
Expected: Compiles with warnings only

**Step 4: Commit**

```bash
git add rust-server/src/main.rs rust-server/src/db.rs
git commit -m "feat: add read-only stats API endpoints for web stats app"
```

---

### Task 2: Scaffold Vite App

**Files:**
- Create: `web-stats/` directory with Vite + React + TS + Tailwind

**Step 1: Scaffold the project**

```bash
cd /path/to/isometric-game
npm create vite@latest web-stats -- --template react-ts
cd web-stats
npm install
npm install -D tailwindcss @tailwindcss/vite
npm install @tanstack/react-query react-router-dom
```

**Step 2: Configure Tailwind with Vite**

Replace `web-stats/vite.config.ts`:

```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': 'http://localhost:2567',
    },
  },
})
```

Replace `web-stats/src/index.css`:

```css
@import "tailwindcss";
```

**Step 3: Delete boilerplate**

Remove `web-stats/src/App.css` and clear out the default App.tsx content (will be replaced in Task 3).

**Step 4: Commit**

```bash
git add web-stats/
git commit -m "feat: scaffold web-stats Vite app with Tailwind and TanStack Query"
```

---

### Task 3: App Shell — Layout & Navigation

**Files:**
- Create: `web-stats/src/App.tsx`
- Create: `web-stats/src/components/Layout.tsx`
- Create: `web-stats/src/pages/Dashboard.tsx` (placeholder)
- Create: `web-stats/src/pages/OnlinePlayers.tsx` (placeholder)
- Create: `web-stats/src/pages/Leaderboards.tsx` (placeholder)
- Create: `web-stats/src/pages/ItemRegistry.tsx` (placeholder)
- Create: `web-stats/src/api.ts`

**Step 1: Create the API client**

`web-stats/src/api.ts`:

```ts
const BASE = '/api/stats'

export interface Overview {
  online_players: number
  total_characters: number
  total_accounts: number
}

export interface OnlinePlayer {
  name: string
  combat_level: number
  hitpoints_level: number
  combat_skill_level: number
  total_level: number
}

export interface LeaderboardEntry {
  name: string
  combat_level: number
  hitpoints_level: number
  combat_skill_level: number
  total_level: number
  played_time: number
}

export interface Equipment {
  slot_type: string
  attack_level_required: number
  defence_level_required: number
  attack_bonus: number
  strength_bonus: number
  defence_bonus: number
  weapon_type: string
  range: number
}

export interface Item {
  id: string
  display_name: string
  sprite: string
  description: string
  category: string
  max_stack: number
  base_price: number
  sellable: boolean
  equipment: Equipment | null
}

export const api = {
  overview: (): Promise<Overview> => fetch(`${BASE}/overview`).then(r => r.json()),
  online: (): Promise<OnlinePlayer[]> => fetch(`${BASE}/online`).then(r => r.json()),
  leaderboard: (sort = 'combat_level', limit = 50): Promise<LeaderboardEntry[]> =>
    fetch(`${BASE}/leaderboard?sort=${sort}&limit=${limit}`).then(r => r.json()),
  items: (): Promise<Item[]> => fetch(`${BASE}/items`).then(r => r.json()),
}
```

**Step 2: Create the layout component**

`web-stats/src/components/Layout.tsx`:

A sidebar layout with the New Aeven branding and navigation links to Dashboard, Online Players, Leaderboards, Item Registry. Dark theme using Tailwind classes. The sidebar should have:
- Game title "New Aeven" with a subtle gold accent
- Nav links with active state highlighting
- Responsive: collapsible sidebar on mobile

**Step 3: Create placeholder pages**

Each page file (`Dashboard.tsx`, `OnlinePlayers.tsx`, `Leaderboards.tsx`, `ItemRegistry.tsx`) exports a component with just the page title for now.

**Step 4: Wire up App.tsx with routing**

`web-stats/src/App.tsx`:

```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Layout } from './components/Layout'
import { Dashboard } from './pages/Dashboard'
import { OnlinePlayers } from './pages/OnlinePlayers'
import { Leaderboards } from './pages/Leaderboards'
import { ItemRegistry } from './pages/ItemRegistry'

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchInterval: 30000 } },
})

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Dashboard />} />
            <Route path="/players" element={<OnlinePlayers />} />
            <Route path="/leaderboards" element={<Leaderboards />} />
            <Route path="/items" element={<ItemRegistry />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
```

**Step 5: Verify dev server runs**

Run: `cd web-stats && npm run dev`
Expected: App loads with sidebar nav and placeholder content

**Step 6: Commit**

```bash
git add web-stats/
git commit -m "feat: add app shell with layout, routing, and API client"
```

---

### Task 4: Dashboard Page

**Files:**
- Modify: `web-stats/src/pages/Dashboard.tsx`

**Step 1: Implement the dashboard**

Full implementation with:
- Hero section with "New Aeven" title and subtitle "Game World Statistics"
- Three stat cards in a responsive grid: Online Players, Total Characters, Total Accounts
- Each card has an icon, large number, and label
- Cards use dark slate bg with subtle gold border on hover
- Data fetched via TanStack Query using `api.overview()`
- Loading skeleton states
- Auto-refreshes every 15 seconds

Design notes:
- Stat numbers should be large and gold-colored (`#c9a84c`)
- Cards: `bg-[#1a1d28]` with `border border-[#2a2d38]` and `hover:border-[#c9a84c]/30`
- Use a gradient or subtle texture for the hero area

**Step 2: Verify**

Run: `cd web-stats && npm run dev`
Expected: Dashboard shows stat cards (will show 0s unless server is running)

**Step 3: Commit**

```bash
git add web-stats/src/pages/Dashboard.tsx
git commit -m "feat: implement dashboard page with stat cards"
```

---

### Task 5: Online Players Page

**Files:**
- Modify: `web-stats/src/pages/OnlinePlayers.tsx`

**Step 1: Implement the online players page**

Full implementation with:
- Page title "Online Players" with count badge
- Sortable table with columns: Name, Combat Level, Hitpoints, Combat, Total Level
- Click column headers to sort ascending/descending
- Empty state message when no players online: "No adventurers are currently online"
- Auto-refreshes every 15 seconds
- Table rows have subtle hover state

Design notes:
- Table: `bg-[#1a1d28]` with `divide-y divide-[#2a2d38]`
- Header row: `bg-[#141722]` with uppercase small text
- Sortable headers show an arrow indicator

**Step 2: Verify**

Run: `cd web-stats && npm run dev`
Expected: Online players page shows table (empty if no server running)

**Step 3: Commit**

```bash
git add web-stats/src/pages/OnlinePlayers.tsx
git commit -m "feat: implement online players page with sortable table"
```

---

### Task 6: Leaderboards Page

**Files:**
- Modify: `web-stats/src/pages/Leaderboards.tsx`

**Step 1: Implement the leaderboards page**

Full implementation with:
- Page title "Leaderboards"
- Tab bar to switch between: Combat Level, Total Level, Hitpoints, Combat Skill
- Ranked table with columns: Rank, Name, Level/XP, Played Time
- Top 3 get special styling (gold #c9a84c, silver #a8a8a8, bronze #b87333)
- Rank numbers are prominent
- Search/filter input to find a player by name
- Played time formatted as "Xh Ym"

Design notes:
- Tabs: pill-style buttons, active tab has gold bg
- Top 3 rows have a subtle left-border color accent
- Same table styling as Online Players for consistency

**Step 2: Verify**

Run: `cd web-stats && npm run dev`
Expected: Leaderboard page with tabs and ranked table

**Step 3: Commit**

```bash
git add web-stats/src/pages/Leaderboards.tsx
git commit -m "feat: implement leaderboards page with ranking tabs"
```

---

### Task 7: Item Registry Page

**Files:**
- Modify: `web-stats/src/pages/ItemRegistry.tsx`

**Step 1: Implement the item registry page**

Full implementation with:
- Page title "Item Registry" with item count
- Search input (filters by name or ID)
- Category filter buttons: All, Equipment, Consumable, Material, Quest
- Card grid layout (responsive: 1-4 columns)
- Each card shows: display name, category badge, description
- Equipment cards show expanded stats section: slot type, bonuses (attack/strength/defence), level requirements
- Cards sorted alphabetically by display name

Design notes:
- Cards: `bg-[#1a1d28]` with rounded corners and border
- Category badges: colored pills (equipment=blue, consumable=green, material=amber, quest=purple)
- Stat lines use a label: value layout with muted labels
- Equipment bonuses shown with +/- coloring (green for positive, red for negative)
- Level requirements shown in gold

**Step 2: Verify**

Run: `cd web-stats && npm run dev`
Expected: Item registry with filterable card grid

**Step 3: Commit**

```bash
git add web-stats/src/pages/ItemRegistry.tsx
git commit -m "feat: implement item registry page with filters and stat cards"
```

---

### Task 8: Polish & Final Touches

**Files:**
- Modify: various `web-stats/src/` files

**Step 1: Add loading and error states**

- Consistent loading skeleton components across all pages
- Error state with retry button if API calls fail
- Smooth fade-in transitions for data loading

**Step 2: Responsive design pass**

- Sidebar collapses to top nav on mobile
- Tables scroll horizontally on small screens
- Card grids adjust columns properly
- Touch-friendly tap targets

**Step 3: Update web-stats index.html**

Set page title to "New Aeven — World Stats" and update the favicon/meta tags.

**Step 4: Final build check**

Run: `cd web-stats && npm run build`
Expected: Build succeeds with no TypeScript errors

**Step 5: Commit**

```bash
git add web-stats/
git commit -m "feat: add polish, loading states, and responsive design"
```
