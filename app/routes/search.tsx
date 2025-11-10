import * as React from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import * as api from '../ipc-bridge'

export const Route = createFileRoute('/search')({
  component: Search,
})

function Search() {
  const [q, setQ] = React.useState('')
  const [hits, setHits] = React.useState<api.SearchHit[]>([])
  const [loading, setLoading] = React.useState(false)

  async function run() {
    setLoading(true)
    try {
      const res = await api.search(q)
      setHits(res)
    } finally {
      setLoading(false)
    }
  }

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">Search</h1>
      <div className="flex gap-2">
        <input className="border rounded px-3 py-2 w-full" placeholder="Query" value={q} onChange={(e) => setQ(e.target.value)} />
        <button className="px-3 py-2 bg-black text-white rounded" onClick={run} disabled={loading}>
          {loading ? 'Searching…' : 'Search'}
        </button>
      </div>
      <ul className="space-y-3">
        {hits.map((h) => (
          <li key={h.id} className="border rounded p-3">
            <Link to={`/doc/${h.id}`} className="font-medium" dangerouslySetInnerHTML={{ __html: h.title_snip || h.slug }} />
            <div className="text-sm text-gray-600" dangerouslySetInnerHTML={{ __html: h.body_snip }} />
          </li>
        ))}
      </ul>
    </main>
  )
}
