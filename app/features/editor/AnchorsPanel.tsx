import * as React from 'react'
import * as api from '../../ipc-bridge'

type EditorAPI = { insertAnchor: (id?: string) => { id: string; line: number } | null }

type Props = {
  docId: string
  editorApiRef: React.MutableRefObject<EditorAPI | null>
}

export function AnchorsPanel({ docId, editorApiRef }: Props) {
  const [anchors, setAnchors] = React.useState<Array<{ id: string; line: number; created_at: string }>>([])
  const [loading, setLoading] = React.useState(false)

  const load = React.useCallback(async () => {
    setLoading(true)
    try {
      const list = await api.anchorsList(docId)
      setAnchors(list)
    } finally {
      setLoading(false)
    }
  }, [docId])

  React.useEffect(() => {
    load()
  }, [load])

  async function addAnchor() {
    const apiRef = editorApiRef.current
    if (!apiRef) return
    const created = apiRef.insertAnchor?.()
    if (created) {
      await api.anchorsUpsert(docId, created.id, created.line)
      setAnchors((prev) => [{ id: created.id, line: created.line, created_at: new Date().toISOString() }, ...prev])
    }
  }

  async function removeAnchor(id: string) {
    await api.anchorsDelete(id)
    setAnchors((prev) => prev.filter((a) => a.id !== id))
  }

  return (
    <section aria-labelledby="anchors-heading" className="space-y-2">
      <div className="flex items-center justify-between">
        <h2 id="anchors-heading" className="font-semibold">Anchors</h2>
        <button className="px-2 py-1 border rounded" onClick={addAnchor} aria-label="Add Anchor">Add Anchor</button>
      </div>
      {loading ? (
        <div className="text-sm text-gray-600">Loading…</div>
      ) : (
        <ul className="space-y-2">
          {anchors.map((a) => (
            <li key={a.id} className="border rounded p-2 flex items-center justify-between">
              <div className="text-sm">#{a.line} <span className="text-gray-600">{a.id}</span></div>
              <button className="px-2 py-1 border rounded text-red-600" onClick={() => removeAnchor(a.id)} aria-label={`Remove anchor ${a.id}`}>Remove</button>
            </li>
          ))}
          {!anchors.length && <li className="text-sm text-gray-600">No anchors</li>}
        </ul>
      )}
    </section>
  )
}

