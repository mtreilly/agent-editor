import * as React from 'react'
import { useNavigate } from '@tanstack/react-router'
import { getCommands, setCommands, builtinCommands } from './commandBus'
import { listUIContributions } from '../../../src/plugins/host'

export function CommandPalette() {
  const [open, setOpen] = React.useState(false)
  const [q, setQ] = React.useState('')
  const [active, setActive] = React.useState(0)
  const navigate = useNavigate()

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().includes('MAC')
      if ((isMac && e.metaKey && e.key.toLowerCase() === 'k') || (!isMac && e.ctrlKey && e.key.toLowerCase() === 'k')) {
        e.preventDefault()
        setOpen((v) => !v)
      } else if (e.key === 'Escape') {
        setOpen(false)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  React.useEffect(() => {
    const contrib = listUIContributions()
    const pluginCmds = (contrib.commands || []).map((c) => ({ id: c.id, title: c.title, run: () => Promise.resolve(c.run({})) }))
    const cmds = [...builtinCommands((to) => navigate({ to })), ...pluginCmds]
    setCommands(cmds)
  }, [navigate])

  const all = getCommands()
  const items = all.filter((c) => c.title.toLowerCase().includes(q.toLowerCase()))

  if (!open) return null
  return (
    <div className="fixed inset-0 bg-black/30 z-50" onClick={() => setOpen(false)}>
      <div className="mx-auto mt-24 w-full max-w-xl" onClick={(e) => e.stopPropagation()}>
        <div className="bg-white rounded shadow border">
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') setActive((i) => Math.min(i + 1, Math.max(0, items.length - 1)))
              else if (e.key === 'ArrowUp') setActive((i) => Math.max(i - 1, 0))
              else if (e.key === 'Enter') {
                const it = items[active]
                if (it) {
                  Promise.resolve(it.run()).finally(() => setOpen(false))
                }
              }
            }}
            placeholder="Type a command…"
            className="w-full px-3 py-2 border-b rounded-t outline-none"
            aria-label="Command palette input"
          />
          <ul className="max-h-64 overflow-auto">
            {items.length === 0 && <li className="px-3 py-2 text-sm text-gray-500">No matches</li>}
            {items.map((c, i) => (
              <li
                key={c.id}
                className={`px-3 py-2 cursor-pointer ${i === active ? 'bg-blue-50' : ''}`}
                onMouseEnter={() => setActive(i)}
                onClick={() => {
                  Promise.resolve(c.run()).finally(() => setOpen(false))
                }}
              >
                {c.title}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  )
}

