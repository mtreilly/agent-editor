import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { loadUIPlugin, listUIContributions } from '../../src/plugins/host'
import type { PluginContext } from '../../src/plugins/types'
import { ipcCall } from '../../src/ipc/client'

export const Route = createFileRoute('/plugins')({
  component: PluginsPage,
})

function PluginsPage() {
  const [loaded, setLoaded] = React.useState(false)
  const [commands, setCommands] = React.useState<Array<{ id: string; title: string; run: (args: any) => Promise<void> | void }>>([])

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

  return (
    <main className="p-6 space-y-4">
      <h1 className="text-xl font-semibold">Plugins</h1>
      <div className="flex gap-2">
        <button className="px-3 py-2 border rounded" onClick={loadHello} disabled={loaded}>
          {loaded ? 'Loaded' : 'Load Hello World'}
        </button>
      </div>
      {!!commands.length && (
        <section className="space-y-2">
          <h2 className="font-semibold">Commands</h2>
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
    </main>
  )
}

