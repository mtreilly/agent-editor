import * as React from 'react'
import { createFileRoute, useParams } from '@tanstack/react-router'
import * as api from '../ipc-bridge'

export const Route = createFileRoute('/doc/$id')({
  component: DocPage,
})

function DocPage() {
  const { id } = Route.useParams()
  const [doc, setDoc] = React.useState<any>(null)
  const [body, setBody] = React.useState('')
  const [saving, setSaving] = React.useState(false)

  React.useEffect(() => {
    (async () => {
      const d = await api.docsGet(id, true)
      setDoc(d)
      setBody(d.body || '')
    })()
  }, [id])

  async function save() {
    if (!doc) return
    setSaving(true)
    try {
      await api.docsUpdate(doc.id, body, 'edit from UI')
      const d = await api.docsGet(doc.id, true)
      setDoc(d)
    } finally {
      setSaving(false)
    }
  }

  if (!doc) return <main className="p-6">Loading…</main>
  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{doc.title || doc.slug}</h1>
      <textarea className="w-full h-[60vh] border rounded p-3 font-mono text-sm" value={body} onChange={(e) => setBody(e.target.value)} />
      <div>
        <button className="px-3 py-2 bg-black text-white rounded" onClick={save} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
      </div>
    </main>
  )
}

