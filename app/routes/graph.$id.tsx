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

  React.useEffect(() => {
    ;(async () => {
      setLoading(true)
      try {
        const [bl, nb] = await Promise.all([api.graphBacklinks(id), api.graphNeighbors(id, 1)])
        setBacklinks(bl)
        setNeighbors(nb)
      } finally {
        setLoading(false)
      }
    })()
  }, [id])

  return (
    <main className="p-6 space-y-6">
      <h1 className="text-xl font-semibold">Graph</h1>
      {loading ? <div>Loading…</div> : null}
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

