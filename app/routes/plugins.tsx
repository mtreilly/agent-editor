import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { loadUIPlugin, listUIContributions } from '../../src/plugins/host'
import type { PluginContext } from '../../src/plugins/types'
import { ipcCall, pluginsCoreList, pluginsSpawnCore, pluginsShutdownCore } from '../../src/ipc/client'
import { useTranslation } from 'react-i18next'

export const Route = createFileRoute('/plugins')({
  component: PluginsPage,
})

function PluginsPage() {
  const { t } = useTranslation('plugins')
  const [loaded, setLoaded] = React.useState(false)
  const [commands, setCommands] = React.useState<Array<{ id: string; title: string; run: (args: any) => Promise<void> | void }>>([])
  const [core, setCore] = React.useState<Array<{ name: string; pid?: number; running: boolean }>>([])

  const ctx = React.useMemo<PluginContext>(
    () => ({
      version: 'v1',
      permissions: {},
      ipc: {
        call: ipcCall as any,
        on: () => () => {},
      },
    }),
    [],
  )

  async function loadHello() {
    await loadUIPlugin('/plugins/hello-world/index.ts', ctx)
    const contrib = listUIContributions()
    setCommands(contrib.commands || [])
    setLoaded(true)
  }

  const hasTauri = React.useMemo(() => typeof window !== 'undefined' && typeof (window as any).__TAURI__ !== 'undefined', [])

  async function refreshCore() {
    try { setCore(await pluginsCoreList()) } catch { setCore([]) }
  }

  React.useEffect(() => { refreshCore() }, [])

  async function spawnEcho() {
    await pluginsSpawnCore('echo', 'node', ['plugins/echo-core/echo.js'])
    await refreshCore()
  }

  async function stopEcho() {
    await pluginsShutdownCore('echo')
    await refreshCore()
  }

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">{t('title')}</h1>
      <div className="flex gap-2">
        <button className="px-3 py-2 border rounded" onClick={loadHello} disabled={loaded}>
          {loaded ? t('button.loaded') : t('button.loadHello')}
        </button>
      </div>
      {!!commands.length && (
        <section className="space-y-2">
          <h2 className="font-semibold">{t('commands')}</h2>
          <ul className="space-y-2">
            {commands.map((c) => (
              <li key={c.id}>
                <button
                  className="px-2 py-1 border rounded"
                  onClick={() => c.run({})}
                  aria-label={`Run ${c.title}`}
                >
                  {c.title}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="space-y-2">
        <h2 className="font-semibold">{t('coreTitle', { defaultValue: 'Core Plugins' })}</h2>
        {!hasTauri && (
          <div className="text-xs text-amber-700">{t('label.notInWeb', { defaultValue: 'Core plugin control not available in web tests' })}</div>
        )}
        <div className="flex gap-2">
          <button className="px-2 py-1 border rounded disabled:opacity-50" onClick={spawnEcho} disabled={!hasTauri}>{t('button.spawnEcho', { defaultValue: 'Spawn Echo' })}</button>
          <button className="px-2 py-1 border rounded disabled:opacity-50" onClick={stopEcho} disabled={!hasTauri}>{t('button.stopEcho', { defaultValue: 'Stop Echo' })}</button>
          <button className="px-2 py-1 border rounded" onClick={refreshCore}>{t('button.refresh', { defaultValue: 'Refresh' })}</button>
        </div>
        <div>
          <h3 className="font-medium text-sm">{t('coreList', { defaultValue: 'Running' })}</h3>
          <ul className="text-sm">
            {core.map((c) => (
              <li key={c.name} className="font-mono">{c.name} — {c.running ? 'running' : 'stopped'}{c.pid ? ` (pid ${c.pid})` : ''}</li>
            ))}
            {!core.length && <li className="text-gray-600 text-xs">{t('none', { ns: 'common' })}</li>}
          </ul>
        </div>
      </section>
    </main>
  )
}
