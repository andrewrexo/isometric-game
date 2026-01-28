import { useState, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api, type Item } from '../api'

const CATEGORIES = ['all', 'equipment', 'consumable', 'material', 'quest'] as const

const categoryBadge: Record<string, string> = {
  equipment: 'bg-blue-500/20 text-blue-400',
  consumable: 'bg-green-500/20 text-green-400',
  material: 'bg-amber-500/20 text-amber-400',
  quest: 'bg-purple-500/20 text-purple-400',
}

function StatLine({ label, value }: { label: string; value: number }) {
  if (value === 0) return null
  const color = value > 0 ? 'text-[#4ade80]' : 'text-[#f87171]'
  const prefix = value > 0 ? '+' : ''
  return (
    <span className={`text-xs ${color}`}>
      {prefix}{value} {label}
    </span>
  )
}

export function ItemRegistry() {
  const { data, isLoading } = useQuery({
    queryKey: ['items'],
    queryFn: api.items,
  })

  const [search, setSearch] = useState('')
  const [category, setCategory] = useState<string>('all')

  const filtered = useMemo(() => {
    if (!data) return []
    const q = search.toLowerCase()
    return [...data]
      .filter(item => {
        if (category !== 'all' && item.category !== category) return false
        if (q && !item.display_name.toLowerCase().includes(q) && !item.id.toLowerCase().includes(q)) return false
        return true
      })
      .sort((a, b) => a.display_name.localeCompare(b.display_name))
  }, [data, search, category])

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-bold text-[#e2e4e9]">Item Registry</h1>
        {data && (
          <span className="rounded-full bg-[#c9a84c] px-3 py-0.5 text-sm font-semibold text-[#0f1117]">
            {filtered.length}
          </span>
        )}
      </div>

      {/* Search */}
      <input
        type="text"
        placeholder="Search items..."
        value={search}
        onChange={e => setSearch(e.target.value)}
        className="w-full max-w-sm rounded-lg border border-[#2a2d38] bg-[#141722] px-4 py-2 text-sm text-[#e2e4e9] placeholder-[#5a5e72] outline-none focus:border-[#c9a84c] transition-colors"
      />

      {/* Category filters */}
      <div className="flex flex-wrap gap-2">
        {CATEGORIES.map(cat => (
          <button
            key={cat}
            onClick={() => setCategory(cat)}
            className={`rounded-full px-4 py-1.5 text-sm font-medium transition-colors ${
              category === cat
                ? 'bg-[#c9a84c] text-[#0f1117]'
                : 'bg-[#1a1d28] text-[#8b8fa3] hover:text-[#e2e4e9] border border-[#2a2d38]'
            }`}
          >
            {cat.charAt(0).toUpperCase() + cat.slice(1)}
          </button>
        ))}
      </div>

      {/* Items grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className="bg-[#1a1d28] rounded-lg border border-[#2a2d38] p-4 space-y-3">
              <div className="h-5 w-32 rounded bg-[#2a2d38] animate-pulse" />
              <div className="h-4 w-20 rounded bg-[#2a2d38] animate-pulse" />
              <div className="h-4 w-full rounded bg-[#2a2d38] animate-pulse" />
            </div>
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <p className="text-center py-12 text-[#8b8fa3]">No items found</p>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {filtered.map(item => (
            <ItemCard key={item.id} item={item} />
          ))}
        </div>
      )}
    </div>
  )
}

function ItemCard({ item }: { item: Item }) {
  const eq = item.equipment

  return (
    <div className="bg-[#1a1d28] rounded-lg border border-[#2a2d38] p-4 hover:border-[#c9a84c]/30 transition-colors space-y-2">
      <p className="font-bold text-[#e2e4e9]">{item.display_name}</p>

      <div className="flex flex-wrap items-center gap-2">
        <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${categoryBadge[item.category] ?? 'bg-[#2a2d38] text-[#8b8fa3]'}`}>
          {item.category}
        </span>
        {eq && (
          <span className="rounded-full bg-[#2a2d38] px-2 py-0.5 text-xs text-[#8b8fa3]">
            {eq.slot_type}
          </span>
        )}
      </div>

      {item.description && (
        <p className="text-sm text-[#8b8fa3] line-clamp-2">{item.description}</p>
      )}

      {item.base_price > 0 && (
        <p className="text-sm text-[#c9a84c]">{item.base_price.toLocaleString()} gold</p>
      )}

      {eq && (
        <div className="space-y-1 pt-1 border-t border-[#2a2d38]">
          <div className="flex flex-wrap gap-x-3 gap-y-0.5">
            <StatLine label="Attack" value={eq.attack_bonus} />
            <StatLine label="Strength" value={eq.strength_bonus} />
            <StatLine label="Defence" value={eq.defence_bonus} />
          </div>
          {eq.slot_type === 'weapon' && eq.attack_level_required > 1 && (
            <p className="text-xs text-[#c9a84c]">Requires {eq.attack_level_required} Attack</p>
          )}
          {eq.slot_type !== 'weapon' && eq.defence_level_required > 1 && (
            <p className="text-xs text-[#c9a84c]">Requires {eq.defence_level_required} Defence</p>
          )}
        </div>
      )}
    </div>
  )
}
