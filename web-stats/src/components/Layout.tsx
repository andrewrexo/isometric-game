import { useState } from 'react'
import { NavLink, Outlet } from 'react-router-dom'

const navItems = [
  {
    to: '/',
    label: 'Dashboard',
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="1" y="1" width="6.5" height="6.5" rx="1" />
        <rect x="10.5" y="1" width="6.5" height="6.5" rx="1" />
        <rect x="1" y="10.5" width="6.5" height="6.5" rx="1" />
        <rect x="10.5" y="10.5" width="6.5" height="6.5" rx="1" />
      </svg>
    ),
  },
  {
    to: '/players',
    label: 'Online Players',
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="7" cy="5.5" r="3" />
        <path d="M1.5 16.5v-1.5a4 4 0 0 1 4-4h3a4 4 0 0 1 4 4v1.5" />
        <circle cx="13.5" cy="6" r="2" />
        <path d="M14 11c1.7.3 3 1.7 3 3.5v2" />
      </svg>
    ),
  },
  {
    to: '/leaderboards',
    label: 'Leaderboards',
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M5 17V9h3V3l-6 8h4l-1 6z" />
        <rect x="9" y="7" width="4" height="10" rx="0.5" />
        <rect x="14" y="4" width="3" height="13" rx="0.5" />
        <rect x="1" y="10" width="3" height="7" rx="0.5" />
        <line x1="1" y1="17" x2="17" y2="17" />
      </svg>
    ),
  },
  {
    to: '/items',
    label: 'Item Registry',
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M2 4.5L9 1l7 3.5" />
        <path d="M2 4.5v9L9 17l7-3.5v-9" />
        <path d="M2 4.5L9 8l7-3.5" />
        <line x1="9" y1="8" x2="9" y2="17" />
      </svg>
    ),
  },
]

export function Layout() {
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <div className="flex min-h-screen bg-[#0f1117]">
      {/* Mobile overlay */}
      {mobileOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm md:hidden"
          onClick={() => setMobileOpen(false)}
        />
      )}

      {/* Sidebar */}
      <aside
        className={`
          fixed top-0 left-0 z-50 flex h-full w-60 flex-col border-r border-[#2a2d38] bg-[#141722]
          transition-transform duration-300 ease-in-out
          md:translate-x-0 md:static md:z-auto
          ${mobileOpen ? 'translate-x-0' : '-translate-x-full'}
        `}
      >
        {/* Brand */}
        <div className="flex flex-col items-start px-6 pt-7 pb-2">
          <span
            className="text-xl font-bold tracking-[0.25em] text-[#c9a84c]"
            style={{ fontVariant: 'small-caps' }}
          >
            NEW AEVEN
          </span>
          <span className="mt-0.5 text-xs tracking-widest text-[#5a5e72] uppercase">
            World Statistics
          </span>
          <div className="mt-4 h-px w-full bg-gradient-to-r from-[#c9a84c]/40 via-[#2a2d38] to-transparent" />
        </div>

        {/* Nav */}
        <nav className="mt-4 flex flex-1 flex-col gap-1 px-3">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              onClick={() => setMobileOpen(false)}
              className={({ isActive }) =>
                `group flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-200 ${
                  isActive
                    ? 'border-l-2 border-[#c9a84c] bg-[#c9a84c]/10 text-[#c9a84c]'
                    : 'border-l-2 border-transparent text-[#8b8fa3] hover:bg-[#1a1d28] hover:text-[#e2e4e9]'
                }`
              }
            >
              <span className="shrink-0 transition-colors duration-200">{item.icon}</span>
              {item.label}
            </NavLink>
          ))}
        </nav>

        {/* Footer */}
        <div className="px-6 py-4">
          <div className="h-px w-full bg-[#2a2d38]" />
          <p className="mt-3 text-[10px] tracking-wider text-[#5a5e72] uppercase">
            Powered by New Aeven
          </p>
        </div>
      </aside>

      {/* Main content */}
      <div className="flex flex-1 flex-col md:ml-0">
        {/* Mobile header */}
        <header className="sticky top-0 z-30 flex items-center gap-3 border-b border-[#2a2d38] bg-[#141722]/90 px-4 py-3 backdrop-blur md:hidden">
          <button
            onClick={() => setMobileOpen(!mobileOpen)}
            className="flex h-9 w-9 items-center justify-center rounded-lg text-[#8b8fa3] transition-colors hover:bg-[#1a1d28] hover:text-[#e2e4e9]"
            aria-label="Toggle menu"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <line x1="3" y1="5" x2="17" y2="5" />
              <line x1="3" y1="10" x2="17" y2="10" />
              <line x1="3" y1="15" x2="17" y2="15" />
            </svg>
          </button>
          <span className="text-sm font-bold tracking-[0.2em] text-[#c9a84c]" style={{ fontVariant: 'small-caps' }}>
            NEW AEVEN
          </span>
        </header>

        {/* Page content */}
        <main className="flex-1 px-6 py-8 md:px-10 md:py-10">
          <div className="mx-auto max-w-5xl">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  )
}
