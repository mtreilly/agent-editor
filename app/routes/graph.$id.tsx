import * as React from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import * as api from '../ipc-bridge'

export const Route = createFileRoute('/graph/$id')({
  component: GraphPage,
})

function GraphPage() {
  const { id } = Route.useParams()
  const [backlinks, setBacklinks] = React.useState<api.GraphDoc[]>([])
  const [neighbors, setNeighbors] = React.useState<api.GraphDoc[]>([])
  const [loading, setLoading] = React.useState(true)
  const [target, setTarget] = React.useState('')
  const [path, setPath] = React.useState<string[]>([])
  const [pathDocs, setPathDocs] = React.useState<Array<{ id: string; title: string; slug: string }>>([])
  const [depth, setDepth] = React.useState(1)

  React.useEffect(() => {
    ;(async () => {
      setLoading(true)
      try {
        const [bl, nb] = await Promise.all([api.graphBacklinks(id), api.graphNeighbors(id, depth)])
        setBacklinks(bl)
        setNeighbors(nb)
      } finally {
        setLoading(false)
      }
    })()
  }, [id, depth])

  async function computePath() {
    if (!target) return
    const p = await api.graphPath(id, target)
    setPath(p)
    // fetch titles for each id in path
    const docs = await Promise.all(
      p.map(async (docId) => {
        const d = await api.docsGet(docId, false)
        return { id: docId, title: d?.title || '', slug: d?.slug || docId }
      }),
    )
    setPathDocs(docs)
  }

  return (
    <main className="p-6 space-y-6">
      <h1 className="text-xl font-semibold">Graph</h1>
      {loading ? <div>Loading…</div> : null}
      <section className="space-y-2">
        <h2 className="font-semibold">Shortest Path</h2>
        <div className="flex gap-2">
          <input className="border rounded px-3 py-2 w-full" placeholder="Target doc id or slug" value={target} onChange={(e) => setTarget(e.target.value)} />
          <button className="px-3 py-2 border rounded" onClick={computePath} disabled={!target}>Compute</button>
        </div>
        {!!pathDocs.length && (
          <ol className="list-decimal pl-6 space-y-1">
            {pathDocs.map((d) => (
              <li key={d.id}>
                <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
              </li>
            ))}
          </ol>
        )}
      </section>
      <section>
        <h2 className="font-semibold mb-2">Backlinks</h2>
        <ul className="space-y-2">
          {backlinks.map((d) => (
            <li key={d.id} className="border rounded p-2">
              <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
            </li>
          ))}
          {!backlinks.length && <li className="text-sm text-gray-600">No backlinks</li>}
        </ul>
      </section>
      <section>
        <h2 className="font-semibold mb-2">Neighbors</h2>
        <div className="mb-2 flex items-center gap-2 text-sm">
          <label htmlFor="depth">Depth</label>
          <select id="depth" className="border rounded px-2 py-1" value={depth} onChange={(e) => setDepth(parseInt(e.target.value))}>
            <option value={1}>1</option>
            <option value={2}>2</option>
            <option value={3}>3</option>
          </select>
        </div>
        <ul className="space-y-2">
          {neighbors.map((d) => (
            <li key={d.id} className="border rounded p-2">
              <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
            </li>
          ))}
          {!neighbors.length && <li className="text-sm text-gray-600">No neighbors</li>}
        </ul>
      </section>
    </main>
  )
}
