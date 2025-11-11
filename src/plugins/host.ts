// UI Plugin Host scaffolding (v0)
import type { Contributions, PluginV1, PluginContext } from './types'

type Loaded = { name: string; version: string; kind: 'ui'; contributions: Contributions }
const loaded: Loaded[] = []

export async function loadUIPlugin(url: string, ctx: PluginContext): Promise<Loaded> {
  const mod = await import(/* @vite-ignore */ url)
  const plugin: PluginV1 = mod.default
  if (!plugin || plugin.kind !== 'ui') throw new Error('Invalid UI plugin')
  const contributions = await plugin.activate(ctx)
  const item = { name: plugin.name, version: plugin.version, kind: 'ui' as const, contributions }
  loaded.push(item)
  return item
}

export function listUIContributions(): Contributions {
  const out: Contributions = {}
  for (const l of loaded) {
    if (l.contributions.commands) {
      out.commands = (out.commands || []).concat(l.contributions.commands)
    }
    if (l.contributions.views) {
      out.views = (out.views || []).concat(l.contributions.views)
    }
    if (l.contributions.renderers) {
      out.renderers = (out.renderers || []).concat(l.contributions.renderers)
    }
  }
  return out
}

