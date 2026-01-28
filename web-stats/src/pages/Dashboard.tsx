import { useQuery } from '@tanstack/react-query'
import { api } from '../api'

export function Dashboard() {
  const { data, isLoading } = useQuery({
    queryKey: ['overview'],
    queryFn: api.overview,
    refetchInterval: 15000,
  })

  return (
    <div className="space-y-8">
      {/* Hero */}
      <div className="rounded-lg bg-gradient-to-b from-[#1a1d28] to-[#0f1117] px-8 py-12 text-center">
        <h1 className="text-5xl font-bold text-[#c9a84c]">New Aeven</h1>
        <p className="mt-2 text-lg text-[#8b8fa3]">Game World Statistics</p>
      </div>

      {/* Stat Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {isLoading ? (
          <>
            <SkeletonCard />
            <SkeletonCard />
            <SkeletonCard />
          </>
        ) : (
          <>
            <StatCard
              label="Online Players"
              value={data?.online_players ?? 0}
              indicator
            />
            <StatCard
              label="Total Characters"
              value={data?.total_characters ?? 0}
            />
            <StatCard
              label="Total Accounts"
              value={data?.total_accounts ?? 0}
            />
          </>
        )}
      </div>
    </div>
  )
}

function StatCard({ label, value, indicator }: { label: string; value: number; indicator?: boolean }) {
  return (
    <div className="bg-[#1a1d28] border border-[#2a2d38] rounded-lg p-6 hover:border-[#c9a84c]/30 transition-colors duration-200">
      <div className="flex items-center gap-2">
        {indicator && <span className="inline-block h-2.5 w-2.5 rounded-full bg-[#4ade80] shadow-[0_0_6px_#4ade80]" />}
        <span className="text-sm text-[#8b8fa3]">{label}</span>
      </div>
      <p className="mt-3 text-4xl font-bold text-[#c9a84c]">{value.toLocaleString()}</p>
    </div>
  )
}

function SkeletonCard() {
  return (
    <div className="bg-[#1a1d28] border border-[#2a2d38] rounded-lg p-6">
      <div className="h-4 w-28 rounded bg-[#2a2d38] animate-pulse" />
      <div className="mt-4 h-10 w-20 rounded bg-[#2a2d38] animate-pulse" />
    </div>
  )
}
