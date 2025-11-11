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

  const load = React.useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const list = await api.aiProvidersList()
      setProviders(list)
      const keyStates: Record<string, { has: boolean; value: string }> = {}
      for (const p of list) {
        if (p.kind === 'remote') {
          const st = await api.aiProviderKeyGet(p.name)
          keyStates[p.name] = { has: !!st?.has_key, value: '' }
        }
      }
      setKeys(keyStates)
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

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{t('providers')}</h1>
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
          {providers.map((p) => (
            <tr key={p.name} className="border-b">
              <td className="py-2 pr-2 font-mono">{p.name}</td>
              <td className="py-2 pr-2">{p.kind}</td>
              <td className="py-2 pr-2">{p.enabled ? t('yes', { ns: 'common' }) : t('no', { ns: 'common' })}</td>
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
                    <button className="px-2 py-1 border rounded" onClick={() => saveKey(p.name)}>{t('button.save')}</button>
                    {keys[p.name]?.has && <span className="text-xs text-gray-600">{t('label.keySet')}</span>}
                  </div>
                )}
              </td>
            </tr>
          ))}
          {!providers.length && !loading && (
            <tr><td className="py-4 text-gray-600" colSpan={4}>{t('status.noProviders')}</td></tr>
          )}
        </tbody>
      </table>
    </main>
  )
}
