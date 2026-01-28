import { useState, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api } from '../api'

const TABS = [
  { label: 'Combat Level', sort: 'combat_level', field: 'combat_level' as const },
  { label: 'Total Level', sort: 'total_level', field: 'total_level' as const },
  { label: 'Hitpoints', sort: 'hitpoints_level', field: 'hitpoints_level' as const },
  { label: 'Combat Skill', sort: 'combat_skill', field: 'combat_skill_level' as const },
]

function formatTime(seconds: number) {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h}h ${m}m`
}

export function Leaderboards() {
  const [activeTab, setActiveTab] = useState(0)
  const [search, setSearch] = useState('')

  const tab = TABS[activeTab]

  const { data, isLoading } = useQuery({
    queryKey: ['leaderboard', tab.sort],
    queryFn: () => api.leaderboard(tab.sort, 100),
  })

  const filtered = useMemo(() => {
    if (!data) return []
    if (!search) return data
    const q = search.toLowerCase()
    return data.filter(e => e.name.toLowerCase().includes(q))
  }, [data, search])

  const rankStyle = (rank: number) => {
    if (rank === 1) return 'border-l-4 border-[#c9a84c]'
    if (rank === 2) return 'border-l-4 border-[#a8a8a8]'
    if (rank === 3) return 'border-l-4 border-[#b87333]'
    return ''
  }

  const rankColor = (rank: number) => {
    if (rank === 1) return 'text-[#c9a84c]'
    if (rank === 2) return 'text-[#a8a8a8]'
    if (rank === 3) return 'text-[#b87333]'
    return 'text-[#5a5e72]'
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-[#e2e4e9]">Leaderboards</h1>

      {/* Tabs */}
      <div className="flex flex-wrap gap-2">
        {TABS.map((t, i) => (
          <button
            key={t.sort}
            onClick={() => setActiveTab(i)}
            className={`rounded-full px-4 py-2 text-sm font-medium transition-colors ${
              i === activeTab
                ? 'bg-[#c9a84c] text-[#0f1117]'
                : 'bg-[#1a1d28] text-[#8b8fa3] hover:text-[#e2e4e9]'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <input
        type="text"
        placeholder="Search player..."
        value={search}
        onChange={e => setSearch(e.target.value)}
        className="w-full max-w-sm rounded-lg border border-[#2a2d38] bg-[#141722] px-4 py-2 text-sm text-[#e2e4e9] placeholder-[#5a5e72] outline-none focus:border-[#c9a84c] transition-colors"
      />

      {/* Table */}
      <div className="bg-[#1a1d28] rounded-lg border border-[#2a2d38] overflow-hidden">
        <table className="w-full">
          <thead>
            <tr className="bg-[#141722]">
              <th className="px-4 py-3 text-left text-xs uppercase tracking-wider text-[#8b8fa3] w-16">Rank</th>
              <th className="px-4 py-3 text-left text-xs uppercase tracking-wider text-[#8b8fa3]">Player</th>
              <th className="px-4 py-3 text-left text-xs uppercase tracking-wider text-[#8b8fa3]">Level</th>
              <th className="px-4 py-3 text-left text-xs uppercase tracking-wider text-[#8b8fa3]">Played Time</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 8 }).map((_, i) => (
                <tr key={i} className="border-b border-[#2a2d38]">
                  <td className="px-4 py-3"><div className="h-4 w-8 rounded bg-[#2a2d38] animate-pulse" /></td>
                  <td className="px-4 py-3"><div className="h-4 w-24 rounded bg-[#2a2d38] animate-pulse" /></td>
                  <td className="px-4 py-3"><div className="h-4 w-12 rounded bg-[#2a2d38] animate-pulse" /></td>
                  <td className="px-4 py-3"><div className="h-4 w-16 rounded bg-[#2a2d38] animate-pulse" /></td>
                </tr>
              ))
            ) : filtered.length === 0 ? (
              <tr>
                <td colSpan={4} className="px-4 py-12 text-center text-[#8b8fa3]">
                  No players found
                </td>
              </tr>
            ) : (
              filtered.map((entry, i) => {
                const rank = i + 1
                return (
                  <tr key={entry.name} className={`border-b border-[#2a2d38] hover:bg-[#141722] transition-colors ${rankStyle(rank)}`}>
                    <td className={`px-4 py-3 font-mono font-bold ${rankColor(rank)}`}>{rank}</td>
                    <td className="px-4 py-3 text-[#e2e4e9]">{entry.name}</td>
                    <td className="px-4 py-3 font-mono text-[#e2e4e9]">{entry[tab.field]}</td>
                    <td className="px-4 py-3 text-[#8b8fa3]">{formatTime(entry.played_time)}</td>
                  </tr>
                )
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
