import { invoke } from '@tauri-apps/api/core'

function hasTauri() {
  return typeof window !== 'undefined' && typeof (window as any).__TAURI__ !== 'undefined'
}

async function safeInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (hasTauri()) return invoke<T>(cmd, args)
  // Browser (tests/dev web) fallbacks for UI rendering only
  switch (cmd) {
    case 'docs_get': {
      const id = args?.docId
      return { id, repo_id: '', slug: id, title: id } as any as T
    }
    case 'graph_backlinks':
    case 'graph_neighbors':
    case 'graph_related':
      return [] as any as T
    case 'graph_path': {
      const start = args?.startId ?? ''
      const end = args?.endId ?? ''
      return [start, end] as any as T
    }
    case 'search':
      return [] as any as T
    case 'anchors_upsert':
      return { ok: true } as any as T
    case 'anchors_list':
      return [] as any as T
    case 'anchors_delete':
      return { deleted: true } as any as T
    default:
      throw new Error(`Not in Tauri environment: ${cmd}`)
  }
}

export type SearchHit = { id: string; slug: string; title_snip: string; body_snip: string; rank: number }

export const reposAdd = (path: string, name?: string, include?: string[], exclude?: string[]) =>
  safeInvoke<{ repo_id: string }>('repos_add', { path, name, include, exclude })

export const reposList = () => safeInvoke<Array<{ id: string; name: string; path: string }>>('repos_list')

export const reposInfo = (id_or_name: string) => safeInvoke<any>('repos_info', { idOrName: id_or_name })

export const reposRemove = (id_or_name: string) => safeInvoke<{ removed: boolean }>('repos_remove', { idOrName: id_or_name })

export const scanRepo = (
  repoPath: string,
  filters?: { include?: string[]; exclude?: string[] },
  watch?: boolean,
  debounce?: number,
) => safeInvoke<{ job_id: string; files_scanned: number; docs_added: number; errors: number }>('scan_repo', { repoPath, filters, watch, debounce })

export const docsCreate = (repo_id: string, slug: string, title: string, body: string) =>
  safeInvoke<{ doc_id: string }>('docs_create', { payload: { repo_id, slug, title, body } })

export const docsUpdate = (doc_id: string, body: string, message?: string) =>
  safeInvoke<{ version_id: string }>('docs_update', { payload: { doc_id, body, message } })

export const docsGet = (doc_id: string, content?: boolean) => safeInvoke<any>('docs_get', { docId: doc_id, content })

export const docsDelete = (doc_id: string) => safeInvoke<{ deleted: boolean }>('docs_delete', { docId: doc_id })

export const search = (query: string, repo_id?: string, limit = 50, offset = 0) =>
  safeInvoke<SearchHit[]>('search', { repoId: repo_id, query, limit, offset })

export const serveApiStart = (port?: number) => safeInvoke<void>('serve_api_start', { port })

export type GraphDoc = { id: string; slug: string; title: string }
export const graphBacklinks = (doc_id: string) => safeInvoke<GraphDoc[]>('graph_backlinks', { docId: doc_id })
export const graphNeighbors = (doc_id: string, depth = 1) => safeInvoke<GraphDoc[]>('graph_neighbors', { docId: doc_id, depth })
export const graphRelated = (doc_id: string) => safeInvoke<GraphDoc[]>('graph_related', { docId: doc_id })

const _graphPathCache = new Map<string, Promise<string[]>>()
export const graphPath = (start_id: string, end_id: string) => {
  const key = `${start_id}->${end_id}`
  if (_graphPathCache.has(key)) return _graphPathCache.get(key) as Promise<string[]>
  const p = safeInvoke<string[]>('graph_path', { startId: start_id, endId: end_id })
  _graphPathCache.set(key, p)
  return p
}

export const aiRun = (provider: string, doc_id: string, prompt: string, anchor_id?: string) =>
  safeInvoke<{ trace_id: string; text: string }>('ai_run', { provider, docId: doc_id, anchorId: anchor_id, prompt })

export const anchorsUpsert = (doc_id: string, anchor_id: string, line: number) =>
  safeInvoke<{ ok: boolean }>('anchors_upsert', { docId: doc_id, anchorId: anchor_id, line })

export const anchorsList = (doc_id: string) =>
  safeInvoke<Array<{ id: string; line: number; created_at: string }>>('anchors_list', { docId: doc_id })

export const anchorsDelete = (anchor_id: string) =>
  safeInvoke<{ deleted: boolean }>('anchors_delete', { anchorId: anchor_id })

export type Provider = { name: string; kind: 'local' | 'remote'; enabled: boolean }
export const aiProvidersList = () => safeInvoke<Provider[]>('ai_providers_list')
export const aiProvidersEnable = (name: string) => safeInvoke<{ updated: boolean }>('ai_providers_enable', { name })
export const aiProvidersDisable = (name: string) => safeInvoke<{ updated: boolean }>('ai_providers_disable', { name })

export const ipcCall = <T = any>(method: string, params?: any) => safeInvoke<T>(method, params)

export const pluginsList = () => safeInvoke<Array<{ id: string; name: string; version: string; kind: string; enabled: boolean }>>('plugins_list')
export const pluginsInfo = (name: string) => safeInvoke<any>('plugins_info', { name })
export const pluginsEnable = (name: string) => safeInvoke<{ updated: boolean }>('plugins_enable', { name })
export const pluginsDisable = (name: string) => safeInvoke<{ updated: boolean }>('plugins_disable', { name })
export const pluginsRemove = (name: string) => safeInvoke<{ removed: boolean }>('plugins_remove', { name })

export const aiProviderKeySet = (name: string, key: string) => safeInvoke<{ updated: boolean }>('ai_provider_key_set', { name, key })
export const aiProviderKeyGet = (name: string) => safeInvoke<{ has_key: boolean }>('ai_provider_key_get', { name })

export const pluginsSpawnCore = (name: string, exec: string, args?: string[]) =>
  safeInvoke<{ ok?: boolean }>('plugins_spawn_core', { name, exec, args })
export const pluginsShutdownCore = (name: string) => safeInvoke<{ ok?: boolean }>('plugins_shutdown_core', { name })
