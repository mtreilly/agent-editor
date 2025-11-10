import * as React from 'react'
import * as api from '../../ipc-bridge'
import { copyText } from '../../../src/app/clipboard'

type EditorAPI = {
  insertAnchor: (id?: string) => { id: string; line: number } | null
  jumpToAnchor?: (id: string) => boolean
  anchorLinkFor?: (anchorId: string) => string
}

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

  async function jumpTo(id: string) {
    editorApiRef.current?.jumpToAnchor?.(id)
  }

  async function copyLink(id: string) {
    const link = editorApiRef.current?.anchorLinkFor?.(id) || `#${id}`
    await copyText(link)
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
              <div className="flex items-center gap-2">
                <button className="px-2 py-1 border rounded" onClick={() => jumpTo(a.id)} aria-label={`Jump to anchor ${a.id}`}>Jump</button>
                <button className="px-2 py-1 border rounded" onClick={() => copyLink(a.id)} aria-label={`Copy link ${a.id}`}>Copy Link</button>
                <button className="px-2 py-1 border rounded text-red-600" onClick={() => removeAnchor(a.id)} aria-label={`Remove anchor ${a.id}`}>Remove</button>
              </div>
            </li>
          ))}
          {!anchors.length && <li className="text-sm text-gray-600">No anchors</li>}
        </ul>
      )}
    </section>
  )
}
