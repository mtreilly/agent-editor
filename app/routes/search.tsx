import * as React from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { sanitizeHtml } from '../../src/app/sanitize'
import { useTranslation } from 'react-i18next'

export const Route = createFileRoute('/search')({
  component: Search,
})

function Search() {
  const { t } = useTranslation('search')
  const [q, setQ] = React.useState('')
  const [hits, setHits] = React.useState<api.SearchHit[]>([])
  const [loading, setLoading] = React.useState(false)
  const [active, setActive] = React.useState(0)
  const listRef = React.useRef<HTMLUListElement | null>(null)
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
    } else if (e.key === 'Escape') {
      // Clear results focus; keep query intact
      setActive(0)
    }
  }

  React.useEffect(() => {
    // Ensure active item is scrolled into view when navigating with keyboard
    const el = listRef.current?.querySelector<HTMLElement>(`#hit-${active}`)
    el?.scrollIntoView({ block: 'nearest' })
  }, [active])

  return (
    <main className="p-6 space-y-4" onKeyDown={onKeyDown}>
      <h1 className="text-xl font-semibold">{t('title')}</h1>
      <div className="flex gap-2">
        <input
          className="border rounded px-3 py-2 w-full"
          placeholder={t('placeholder.query')}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={onKeyDown}
          aria-label="Search query"
        />
        <button className="px-3 py-2 bg-black text-white rounded" onClick={run} disabled={loading}>
          {loading ? t('status.searching') : t('button.search')}
        </button>
      </div>
      <div className="text-sm text-gray-600" aria-live="polite">
        {hits.length ? t('status.count', { count: hits.length }) : loading ? t('status.searching') : t('status.noResults')}
      </div>
      <ul className="space-y-3" role="listbox" aria-label="Search results" aria-activedescendant={`hit-${active}`} ref={listRef}>
        {hits.map((h, i) => (
          <li
            id={`hit-${i}`}
            key={h.id}
            role="option"
            className={`border rounded p-3 ${i === active ? 'ring-2 ring-blue-500' : ''}`}
            tabIndex={-1}
            aria-selected={i === active}
            onMouseEnter={() => setActive(i)}
          >
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
