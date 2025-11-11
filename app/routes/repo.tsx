import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'

export const Route = createFileRoute('/repo')({
  component: RepoPage,
})

function RepoPage() {
  const { t } = useTranslation('repo')
  const [path, setPath] = React.useState('')
  const [name, setName] = React.useState('')
  const [repos, setRepos] = React.useState<Array<{ id: string; name: string; path: string }>>([])
  const [loading, setLoading] = React.useState(false)
  const [lastEvt, setLastEvt] = React.useState<string>("")
  const [providers, setProviders] = React.useState<Array<api.Provider>>([])
  const [defaults, setDefaults] = React.useState<Record<string, string>>({})
  const [globalDefault, setGlobalDefault] = React.useState<string>('local')

  const load = React.useCallback(async () => {
    const rs = await api.reposList()
    setRepos(rs)
    try {
      const ps = await api.aiProvidersList()
      setProviders(ps)
    } catch {}
    try {
      const g = await api.appSettingsGet('default_provider')
      if (g && typeof g.value === 'string') setGlobalDefault(g.value)
    } catch {}
    const map: Record<string, string> = {}
    for (const r of rs) {
      try {
        const info = await api.reposInfo(r.id)
        const dp = info?.settings?.default_provider || 'local'
        map[r.id] = dp
      } catch {}
    }
    setDefaults(map)
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
      await api.scanRepo(path, {}, true, 300)
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
        <h2 className="text-lg font-semibold">{t('addRepo')}</h2>
        <div className="flex gap-2">
          <input className="border rounded px-3 py-2 w-1/2" placeholder={t('placeholder.path')} value={path} onChange={(e) => setPath(e.target.value)} />
          <input className="border rounded px-3 py-2 w-1/4" placeholder={t('placeholder.name')} value={name} onChange={(e) => setName(e.target.value)} />
          <button className="px-3 py-2 bg-black text-white rounded" onClick={add} disabled={loading}>{t('button.add')}</button>
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="text-lg font-semibold">{t('repositories')}</h2>
        {lastEvt ? (
          <div className="text-xs text-gray-600" role="status" aria-live="polite">
            {t('scanEvent', { event: lastEvt })}
          </div>
        ) : null}
        <ul className="space-y-2">
          {repos.map((r) => (
            <li key={r.id} className="border rounded p-3 flex items-center justify-between">
              <div>
                <div className="font-medium">{r.name || r.path}</div>
                <div className="text-xs text-gray-600">{r.path}</div>
                {!!providers.length && (
                  <div className="mt-2 flex items-center gap-2 text-xs">
                    <span className="text-gray-700">{t('defaultProvider') || 'Default Provider'}:</span>
                    <select
                      className="border rounded px-2 py-1"
                      value={defaults[r.id] || 'local'}
                      onChange={(e) => setDefaults((prev) => ({ ...prev, [r.id]: e.target.value }))}
                    >
                      {providers.map((p) => (
                        <option key={p.name} value={p.name}>{p.name}{p.enabled ? '' : ' (disabled)'}</option>
                      ))}
                    </select>
                    <button
                      className="px-2 py-1 border rounded"
                      onClick={async () => { await api.reposSetDefaultProvider(r.id, defaults[r.id] || 'local'); await load() }}
                    >{t('button.set') || 'Set'}</button>
                    <span className="ml-2 text-gray-600">{t('effectiveProvider', { defaultValue: 'Effective' })}: {(defaults[r.id] && defaults[r.id].length) ? defaults[r.id] : globalDefault}</span>
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                <button className="px-2 py-1 border rounded" onClick={() => scan(r.path)} disabled={loading}>{t('scan')}</button>
                <button className="px-2 py-1 border rounded text-red-600" onClick={() => remove(r.id)} disabled={loading}>{t('remove')}</button>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </main>
  )
}
