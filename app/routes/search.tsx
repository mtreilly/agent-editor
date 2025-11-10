import * as React from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { sanitizeHtml } from '../../src/app/sanitize'

export const Route = createFileRoute('/search')({
  component: Search,
})

function Search() {
  const [q, setQ] = React.useState('')
  const [hits, setHits] = React.useState<api.SearchHit[]>([])
  const [loading, setLoading] = React.useState(false)
  const [active, setActive] = React.useState(0)
  const navigate = useNavigate()

  async function run() {
    setLoading(true)
    try {
      const res = await api.search(q)
      setHits(res)
      setActive(0)
    } finally {
      setLoading(false)
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (!hits.length) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setActive((i) => Math.min(i + 1, hits.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActive((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const h = hits[active]
      if (h) navigate({ to: `/doc/${h.id}` })
    }
  }

  return (
    <main className="p-6 space-y-4" onKeyDown={onKeyDown}>
      <h1 className="text-xl font-semibold">Search</h1>
      <div className="flex gap-2">
        <input
          className="border rounded px-3 py-2 w-full"
          placeholder="Query"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={onKeyDown}
          aria-label="Search query"
        />
        <button className="px-3 py-2 bg-black text-white rounded" onClick={run} disabled={loading}>
          {loading ? 'Searching…' : 'Search'}
        </button>
      </div>
      <div className="text-sm text-gray-600" aria-live="polite">
        {hits.length ? `${hits.length} results` : loading ? 'Searching…' : 'No results'}
      </div>
      <ul className="space-y-3">
        {hits.map((h, i) => (
          <li key={h.id} className={`border rounded p-3 ${i === active ? 'ring-2 ring-blue-500' : ''}`} tabIndex={0} aria-selected={i === active}>
            <Link
              to={`/doc/${h.id}`}
              className="font-medium"
              dangerouslySetInnerHTML={{ __html: sanitizeHtml(h.title_snip || h.slug) }}
            />
            <div className="text-sm text-gray-600" dangerouslySetInnerHTML={{ __html: sanitizeHtml(h.body_snip || '') }} />
          </li>
        ))}
      </ul>
    </main>
  )
}
