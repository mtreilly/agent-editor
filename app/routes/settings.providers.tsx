import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import * as api from '../ipc-bridge'

export const Route = createFileRoute('/settings/providers')({
  component: ProvidersSettings,
})

function ProvidersSettings() {
  const [providers, setProviders] = React.useState<api.Provider[]>([])
  const [loading, setLoading] = React.useState(true)
  const [error, setError] = React.useState<string | null>(null)

  const load = React.useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const list = await api.aiProvidersList()
      setProviders(list)
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

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">Providers</h1>
      {loading && <div className="text-sm text-gray-600">Loading…</div>}
      {error && <div role="alert" className="text-sm text-red-600">{error}</div>}
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr className="text-left border-b">
            <th className="py-2 pr-2">Name</th>
            <th className="py-2 pr-2">Kind</th>
            <th className="py-2 pr-2">Enabled</th>
            <th className="py-2">Action</th>
          </tr>
        </thead>
        <tbody>
          {providers.map((p) => (
            <tr key={p.name} className="border-b">
              <td className="py-2 pr-2 font-mono">{p.name}</td>
              <td className="py-2 pr-2">{p.kind}</td>
              <td className="py-2 pr-2">{p.enabled ? 'Yes' : 'No'}</td>
              <td className="py-2">
                <button
                  className="px-2 py-1 border rounded"
                  onClick={() => toggle(p.name, p.enabled)}
                  aria-label={(p.enabled ? 'Disable' : 'Enable') + ' ' + p.name}
                >
                  {p.enabled ? 'Disable' : 'Enable'}
                </button>
              </td>
            </tr>
          ))}
          {!providers.length && !loading && (
            <tr><td className="py-4 text-gray-600" colSpan={4}>No providers</td></tr>
          )}
        </tbody>
      </table>
    </main>
  )
}

