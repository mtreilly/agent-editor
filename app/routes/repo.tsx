import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { listen } from '@tauri-apps/api/event'

export const Route = createFileRoute('/repo')({
  component: RepoPage,
})

function RepoPage() {
  const [path, setPath] = React.useState('')
  const [name, setName] = React.useState('')
  const [repos, setRepos] = React.useState<Array<{ id: string; name: string; path: string }>>([])
  const [loading, setLoading] = React.useState(false)
  const [lastEvt, setLastEvt] = React.useState<string>("")

  const load = React.useCallback(async () => {
    setRepos(await api.reposList())
  }, [])

  React.useEffect(() => {
    load()
    const un = listen('progress.scan', (e) => {
      try { setLastEvt(JSON.stringify(e.payload)) } catch { /* ignore */ }
    })
    return () => { un.then((x) => x()) }
  }, [load])

  async function add() {
    if (!path) return
    setLoading(true)
    try {
      const { repo_id } = await api.reposAdd(path, name || undefined)
      await api.scanRepo(path, {}, false, 200)
      await load()
    } finally {
      setLoading(false)
    }
  }

  async function scan(p: string) {
    setLoading(true)
    try {
      await api.scanRepo(p, {}, false, 200)
    } finally {
      setLoading(false)
    }
  }

  async function remove(idOrName: string) {
    await api.reposRemove(idOrName)
    await load()
  }

  return (
    <main className="p-6 space-y-6">
      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Add Repository</h2>
        <div className="flex gap-2">
          <input className="border rounded px-3 py-2 w-1/2" placeholder="/absolute/path" value={path} onChange={(e) => setPath(e.target.value)} />
          <input className="border rounded px-3 py-2 w-1/4" placeholder="Optional name" value={name} onChange={(e) => setName(e.target.value)} />
          <button className="px-3 py-2 bg-black text-white rounded" onClick={add} disabled={loading}>Add</button>
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Repositories</h2>
        {lastEvt ? <div className="text-xs text-gray-600">Scan: {lastEvt}</div> : null}
        <ul className="space-y-2">
          {repos.map((r) => (
            <li key={r.id} className="border rounded p-3 flex items-center justify-between">
              <div>
                <div className="font-medium">{r.name || r.path}</div>
                <div className="text-xs text-gray-600">{r.path}</div>
              </div>
              <div className="flex gap-2">
                <button className="px-2 py-1 border rounded" onClick={() => scan(r.path)} disabled={loading}>Scan</button>
                <button className="px-2 py-1 border rounded text-red-600" onClick={() => remove(r.id)} disabled={loading}>Remove</button>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </main>
  )
}
