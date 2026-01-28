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

async function get<T>(path: string): Promise<T> {
  const r = await fetch(`${BASE}${path}`)
  if (!r.ok) throw new Error(`API error: ${r.status}`)
  return r.json()
}

export const api = {
  overview: () => get<Overview>('/overview'),
  online: () => get<OnlinePlayer[]>('/online'),
  leaderboard: (sort = 'combat_level', limit = 50) =>
    get<LeaderboardEntry[]>(`/leaderboard?sort=${sort}&limit=${limit}`),
  items: () => get<Item[]>('/items'),
}
