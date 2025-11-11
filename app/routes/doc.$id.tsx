import * as React from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { AnchorsPanel } from '../features/editor/AnchorsPanel'
import { registerCommands, unregisterCommands } from '../features/commands/commandBus'
import { useTranslation } from 'react-i18next'
const EditorLazy = React.lazy(() => import('../features/editor/Editor').then(m => ({ default: m.Editor })))

export const Route = createFileRoute('/doc/$id')({
  component: DocPage,
})

function DocPage() {
  const { t } = useTranslation(['editor','graph','common'])
  const { id } = Route.useParams()
  const [doc, setDoc] = React.useState<any>(null)
  const [body, setBody] = React.useState('')
  const [saving, setSaving] = React.useState(false)
  const [prompt, setPrompt] = React.useState('Explain this section')
  const [aiOut, setAiOut] = React.useState('')
  const [lastAnchor, setLastAnchor] = React.useState<{ id: string; line: number } | null>(null)
  const [backlinks, setBacklinks] = React.useState<Array<{ id: string; slug: string; title: string }>>([])
  const [neighbors, setNeighbors] = React.useState<Array<{ id: string; slug: string; title: string }>>([])
  const [related, setRelated] = React.useState<Array<{ id: string; slug: string; title: string }>>([])
  const navigate = useNavigate()

  const editorApiRef = React.useRef<{
    insertAnchor: (id?: string) => { id: string; line: number } | null
    jumpToAnchor?: (id: string) => boolean
    anchorLinkFor?: (anchorId: string) => string
  } | null>(null)

  React.useEffect(() => {
    (async () => {
      const d = await api.docsGet(id, true)
      setDoc(d)
      setBody(d.body || '')
      // Load graph info for this doc id/slug
      try {
        const bl = await api.graphBacklinks(id)
        const nb = await api.graphNeighbors(id, 1)
        const rel = await api.graphRelated(id)
        setBacklinks(bl)
        setNeighbors(nb)
        setRelated(rel)
      } catch {}
    })()
  }, [id])

  React.useEffect(() => {
    const container = document.getElementById('editor-container')
    if (!container) return
    const handler = (e: Event) => {
      const any = e as any
      const slug = any?.detail?.slug as string
      if (slug) navigate({ to: `/doc/${slug}` })
    }
    container.addEventListener('wiki:navigate', handler as EventListener)
    return () => container.removeEventListener('wiki:navigate', handler as EventListener)
  }, [navigate])

  // Register command palette action for AI run on this doc
  React.useEffect(() => {
    if (!doc) return
    const owner = `doc-${doc.id}`
    registerCommands(owner, [
      {
        id: 'ai.run.doc',
        title: 'AI: Run on current doc',
        run: async () => {
          try {
            const res = await api.aiRun('default', doc.id, prompt || 'Explain this document')
            alert(res.text)
          } catch (e: any) {
            alert(`AI run failed: ${e?.message || e}`)
          }
        },
      },
    ])
    return () => unregisterCommands(owner)
  }, [doc, prompt])

  // If ?anchor= is present in URL, jump to it once editor is ready
  React.useEffect(() => {
    const qp = new URLSearchParams(window.location.search)
    const anc = qp.get('anchor')
    if (!anc) return
    const id = anc
    const iv = setInterval(() => {
      if (editorApiRef.current?.jumpToAnchor?.(id)) {
        clearInterval(iv)
      }
    }, 100)
    return () => clearInterval(iv)
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
      setLastAnchor(created)
    }
  }

  if (!doc) return <main className="p-6">{t('status.loading')}</main>
  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{doc.title || doc.slug}</h1>
      <div className="border rounded p-2" id="editor-container">
        <React.Suspense fallback={<div className="text-sm text-gray-600">{t('status.loading', { ns: 'editor' })}</div>}>
          <EditorLazy value={body} onChange={setBody} docId={doc.id} onReady={(api) => (editorApiRef.current = api)} />
        </React.Suspense>
      </div>
      <div>
        <button className="px-3 py-2 bg-black text-white rounded" onClick={save} disabled={saving}>{saving ? t('status.loading', { ns: 'editor' }) : t('button.save', { ns: 'editor' })}</button>
        <button className="ml-2 px-3 py-2 border rounded" onClick={insertAnchor}>{t('button.insertAnchor', { ns: 'editor' })}</button>
        {lastAnchor && (
          <span className="ml-3 text-xs text-gray-600">{t('label.lastAnchor', { ns: 'editor', id: lastAnchor.id, line: lastAnchor.line })}</span>
        )}
      </div>
      <AnchorsPanel docId={doc.id} editorApiRef={editorApiRef as any} />
      <div className="space-y-2">
        <div className="flex gap-2">
          <input className="border rounded px-3 py-2 w-full" placeholder={t('placeholder.prompt', { ns: 'editor' })} value={prompt} onChange={(e) => setPrompt(e.target.value)} />
          <button className="px-3 py-2 border rounded" onClick={runAI}>{t('button.runAI', { ns: 'editor' })}</button>
          <button className="px-3 py-2 border rounded" onClick={async () => {
            if (!doc || !lastAnchor) return
            const res = await api.aiRun('local', doc.id, prompt, lastAnchor.id)
            setAiOut(res.text)
          }} disabled={!lastAnchor}>{t('button.runAIAnchor', { ns: 'editor' })}</button>
        </div>
        {aiOut && (
          <pre className="border rounded p-3 whitespace-pre-wrap text-sm">{aiOut}</pre>
        )}
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 pt-4">
        <section>
          <h2 className="font-semibold mb-2">{t('backlinks', { ns: 'graph' })}</h2>
          <ul className="space-y-2">
            {backlinks.map((d) => (
              <li key={d.id}>
                <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
              </li>
            ))}
            {!backlinks.length && <li className="text-sm text-gray-600">{t('none', { ns: 'common' })}</li>}
          </ul>
        </section>
        <section>
          <h2 className="font-semibold mb-2">{t('neighbors', { ns: 'graph' })}</h2>
          <ul className="space-y-2">
            {neighbors.map((d) => (
              <li key={d.id}>
                <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
              </li>
            ))}
            {!neighbors.length && <li className="text-sm text-gray-600">{t('none', { ns: 'common' })}</li>}
          </ul>
        </section>
        <section>
          <h2 className="font-semibold mb-2">{t('related', { ns: 'graph' })}</h2>
          <ul className="space-y-2">
            {related.map((d) => (
              <li key={d.id}>
                <Link to={`/doc/${d.id}`}>{d.title || d.slug}</Link>
              </li>
            ))}
            {!related.length && <li className="text-sm text-gray-600">{t('none', { ns: 'common' })}</li>}
          </ul>
        </section>
      </div>
    </main>
  )
}
