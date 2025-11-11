import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import * as api from '../ipc-bridge'
import { useTranslation } from 'react-i18next'

export const Route = createFileRoute('/settings/providers')({
  component: ProvidersSettings,
})

function ProvidersSettings() {
  const { t } = useTranslation(['settings','common'])
  const [providers, setProviders] = React.useState<api.Provider[]>([])
  const [loading, setLoading] = React.useState(true)
  const [error, setError] = React.useState<string | null>(null)
  const [keys, setKeys] = React.useState<Record<string, { has: boolean; value: string }>>({})
  const [globalDefault, setGlobalDefault] = React.useState<string>('local')
  const [models, setModels] = React.useState<Record<string, string>>({})

  const load = React.useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const list = await api.aiProvidersList()
      setProviders(list)
      try {
        const g = await api.appSettingsGet('default_provider')
        if (g && g.value && typeof g.value === 'string') setGlobalDefault(g.value)
      } catch {}
      const keyStates: Record<string, { has: boolean; value: string }> = {}
      const modelStates: Record<string, string> = {}
      for (const p of list) {
        if (p.kind === 'remote') {
          const st = await api.aiProviderKeyGet(p.name)
          keyStates[p.name] = { has: !!st?.has_key, value: '' }
          try {
            const mg = await api.aiProviderModelGet(p.name)
            modelStates[p.name] = mg?.model || ''
          } catch {}
        }
      }
      setKeys(keyStates)
      setModels(modelStates)
    } catch (e: any) {
      setError(e?.message || 'Failed to load providers')
    } finally {
      setLoading(false)
    }
  }, [])

  React.useEffect(() => { load() }, [load])

  async function toggle(name: string, enabled: boolean) {
    try {
      if (enabled) await api.aiProvidersDisable(name)
      else await api.aiProvidersEnable(name)
      await load()
    } catch (e) {
      console.error(e)
    }
  }

  async function saveKey(name: string) {
    const v = keys[name]?.value || ''
    if (!v) return
    await api.aiProviderKeySet(name, v)
    setKeys((prev) => ({ ...prev, [name]: { has: true, value: '' } }))
  }

  async function testProvider(name: string) {
    try {
      const res = await api.aiProviderTest(name, 'hello')
      alert(`Provider ${name}: ${JSON.stringify(res)}`)
    } catch (e: any) {
      alert(`Provider ${name} test failed: ${e?.message || e}`)
    }
  }

  async function saveModel(name: string) {
    const v = models[name] || ''
    await api.aiProviderModelSet(name, v)
  }

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{t('providers')}</h1>
      <section className="space-y-2">
        <div className="flex items-center gap-2">
          <span className="text-sm">{t('globalDefaultProvider') || 'Global default provider'}:</span>
          <select className="border rounded px-2 py-1" value={globalDefault} onChange={(e) => setGlobalDefault(e.target.value)}>
            {providers.map((p) => (
              <option key={p.name} value={p.name}>{p.name}{p.enabled ? '' : ' (disabled)'}</option>
            ))}
          </select>
          <button className="px-2 py-1 border rounded" onClick={async () => { await api.appSettingsSet('default_provider', globalDefault); await load() }}>{t('button.save')}</button>
        </div>
      </section>
      {loading && <div className="text-sm text-gray-600">{t('status.loading')}</div>}
      {error && <div role="alert" className="text-sm text-red-600">{t('error.loadProviders')}</div>}
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr className="text-left border-b">
            <th className="py-2 pr-2">{t('th.name')}</th>
            <th className="py-2 pr-2">{t('th.kind')}</th>
            <th className="py-2 pr-2">{t('th.enabled')}</th>
            <th className="py-2">{t('th.action')}</th>
          </tr>
        </thead>
        <tbody>
          {providers.map((p) => {
            const hasKey = !!keys[p.name]?.has
            const allowed = p.enabled && (p.kind !== 'remote' || hasKey)
            return (
            <tr key={p.name} className="border-b">
              <td className="py-2 pr-2 font-mono">{p.name}</td>
              <td className="py-2 pr-2">{p.kind}</td>
              <td className="py-2 pr-2">
                {p.enabled ? t('yes', { ns: 'common' }) : t('no', { ns: 'common' })}
                {p.kind === 'remote' && (
                  <>
                    {!hasKey && <span className="ml-2 text-xs text-amber-700" aria-label={t('label.missingKey') as string}>{t('label.missingKey')}</span>}
                    {!p.enabled && <span className="ml-2 text-xs text-red-700" aria-label={t('label.disabled') as string}>{t('label.disabled')}</span>}
                    {allowed && <span className="ml-2 text-xs text-green-700" aria-label={t('label.allowed') as string}>{t('label.allowed')}</span>}
                  </>
                )}
              </td>
              <td className="py-2">
                <button
                  className="px-2 py-1 border rounded"
                  onClick={() => toggle(p.name, p.enabled)}
                  aria-label={(p.enabled ? t('button.disable') : t('button.enable')) + ' ' + p.name}
                >
                  {p.enabled ? t('button.disable') : t('button.enable')}
                </button>
                {p.kind === 'remote' && (
                  <div className="mt-2 flex items-center gap-2">
                    <input
                      type="password"
                      placeholder={t('apiKey')}
                      className="border rounded px-2 py-1"
                      value={keys[p.name]?.value || ''}
                      onChange={(e) => setKeys((prev) => ({ ...prev, [p.name]: { ...(prev[p.name] || { has: false, value: '' }), value: e.target.value } }))}
                    />
                    <button className="px-2 py-1 border rounded disabled:opacity-50" disabled={!keys[p.name]?.value} title={!keys[p.name]?.value ? (t('hint.setKey') as string) : undefined} onClick={() => saveKey(p.name)}>{t('button.save')}</button>
                    <button className="px-2 py-1 border rounded disabled:opacity-50" disabled={!allowed} title={!allowed ? (t('hint.enableProvider') as string) : undefined} onClick={() => testProvider(p.name)}>{t('button.test') || 'Test'}</button>
                    {keys[p.name]?.has && <span className="text-xs text-gray-600">{t('label.keySet')}</span>}
                  </div>
                )}
                {p.name === 'openrouter' && (
                  <div className="mt-2 flex items-center gap-2">
                    <input
                      type="text"
                      placeholder={t('modelPlaceholder', { defaultValue: 'openrouter/auto' })}
                      className="border rounded px-2 py-1"
                      value={models[p.name] ?? ''}
                      onChange={(e) => setModels((prev) => ({ ...prev, [p.name]: e.target.value }))}
                    />
                    <button className="px-2 py-1 border rounded disabled:opacity-50" disabled={!allowed} title={!allowed ? (t('hint.enableProvider') as string) : undefined} onClick={() => saveModel(p.name)}>{t('button.saveModel', { defaultValue: 'Save Model' })}</button>
                  </div>
                )}
              </td>
            </tr>
          )})}
          {!providers.length && !loading && (
            <tr><td className="py-4 text-gray-600" colSpan={4}>{t('status.noProviders')}</td></tr>
          )}
        </tbody>
      </table>
    </main>
  )
}
