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
