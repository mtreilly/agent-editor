import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { Editor } from '../features/editor/Editor'

export const Route = createFileRoute('/doc/$id')({
  component: DocPage,
})

function DocPage() {
  const { id } = Route.useParams()
  const [doc, setDoc] = React.useState<any>(null)
  const [body, setBody] = React.useState('')
  const [saving, setSaving] = React.useState(false)
  const [prompt, setPrompt] = React.useState('Explain this section')
  const [aiOut, setAiOut] = React.useState('')

  const editorApiRef = React.useRef<{ insertAnchor: (id?: string) => { id: string; line: number } | null } | null>(null)

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

  async function runAI() {
    if (!doc) return
    const res = await api.aiRun('local', doc.id, prompt)
    setAiOut(res.text)
  }

  async function insertAnchor() {
    if (!doc || !editorApiRef.current) return
    const created = editorApiRef.current.insertAnchor?.()
    if (created) {
      await api.anchorsUpsert(doc.id, created.id, created.line)
    }
  }

  if (!doc) return <main className="p-6">Loading…</main>
  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{doc.title || doc.slug}</h1>
      <div className="border rounded p-2">
        <Editor value={body} onChange={setBody} docId={doc.id} onReady={(api) => (editorApiRef.current = api)} />
      </div>
      <div>
        <button className="px-3 py-2 bg-black text-white rounded" onClick={save} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
        <button className="ml-2 px-3 py-2 border rounded" onClick={insertAnchor}>Insert Anchor</button>
      </div>
      <div className="space-y-2">
        <div className="flex gap-2">
          <input className="border rounded px-3 py-2 w-full" placeholder="Prompt" value={prompt} onChange={(e) => setPrompt(e.target.value)} />
          <button className="px-3 py-2 border rounded" onClick={runAI}>Run AI</button>
        </div>
        {aiOut && (
          <pre className="border rounded p-3 whitespace-pre-wrap text-sm">{aiOut}</pre>
        )}
      </div>
    </main>
  )
}
