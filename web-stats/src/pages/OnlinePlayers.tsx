import { useState, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api, type OnlinePlayer } from '../api'

type SortKey = keyof OnlinePlayer
type SortDir = 'asc' | 'desc'

export function OnlinePlayers() {
  const { data, isLoading } = useQuery({
    queryKey: ['online'],
    queryFn: api.online,
    refetchInterval: 15000,
  })

  const [sortKey, setSortKey] = useState<SortKey>('combat_level')
  const [sortDir, setSortDir] = useState<SortDir>('desc')

  const sorted = useMemo(() => {
    if (!data) return []
    return [...data].sort((a, b) => {
      const av = a[sortKey]
      const bv = b[sortKey]
      if (av < bv) return sortDir === 'asc' ? -1 : 1
      if (av > bv) return sortDir === 'asc' ? 1 : -1
      return 0
    })
  }, [data, sortKey, sortDir])

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir(d => (d === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortKey(key)
      setSortDir('desc')
    }
  }

  const columns: { key: SortKey; label: string }[] = [
    { key: 'name', label: 'Name' },
    { key: 'combat_level', label: 'Combat Lv' },
    { key: 'hitpoints_level', label: 'Hitpoints' },
    { key: 'combat_skill_level', label: 'Combat' },
    { key: 'total_level', label: 'Total Lv' },
  ]

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-bold text-[#e2e4e9]">Online Players</h1>
        {data && (
          <span className="rounded-full bg-[#c9a84c] px-3 py-0.5 text-sm font-semibold text-[#0f1117]">
            {data.length}
          </span>
        )}
      </div>

      <div className="bg-[#1a1d28] rounded-lg border border-[#2a2d38] overflow-hidden">
        <table className="w-full">
          <thead>
            <tr className="bg-[#141722]">
              {columns.map(col => (
                <th
                  key={col.key}
                  onClick={() => toggleSort(col.key)}
                  className="cursor-pointer px-4 py-3 text-left text-xs uppercase tracking-wider text-[#8b8fa3] select-none hover:text-[#e2e4e9] transition-colors"
                >
                  {col.label}
                  {sortKey === col.key && (
                    <span className="ml-1">{sortDir === 'asc' ? '▲' : '▼'}</span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 6 }).map((_, i) => (
                <tr key={i} className="border-b border-[#2a2d38]">
                  {columns.map(col => (
                    <td key={col.key} className="px-4 py-3">
                      <div className="h-4 w-16 rounded bg-[#2a2d38] animate-pulse" />
                    </td>
                  ))}
                </tr>
              ))
            ) : sorted.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="px-4 py-12 text-center text-[#8b8fa3]">
                  No adventurers are currently online
                </td>
              </tr>
            ) : (
              sorted.map(player => (
                <tr key={player.name} className="border-b border-[#2a2d38] hover:bg-[#141722] transition-colors">
                  <td className="px-4 py-3 text-[#e2e4e9]">{player.name}</td>
                  <td className="px-4 py-3 font-mono text-[#e2e4e9]">{player.combat_level}</td>
                  <td className="px-4 py-3 font-mono text-[#e2e4e9]">{player.hitpoints_level}</td>
                  <td className="px-4 py-3 font-mono text-[#e2e4e9]">{player.combat_skill_level}</td>
                  <td className="px-4 py-3 font-mono text-[#e2e4e9]">{player.total_level}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
