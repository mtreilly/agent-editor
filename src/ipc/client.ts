import { invoke } from '@tauri-apps/api/core'

export type SearchHit = { id: string; slug: string; title_snip: string; body_snip: string; rank: number }

export const reposAdd = (path: string, name?: string, include?: string[], exclude?: string[]) =>
  invoke<{ repo_id: string }>('repos_add', { path, name, include, exclude })

export const reposList = () => invoke<Array<{ id: string; name: string; path: string }>>('repos_list')

export const reposInfo = (id_or_name: string) => invoke<any>('repos_info', { idOrName: id_or_name })

export const reposRemove = (id_or_name: string) => invoke<{ removed: boolean }>('repos_remove', { idOrName: id_or_name })

export const scanRepo = (
  repoPath: string,
  filters?: { include?: string[]; exclude?: string[] },
  watch?: boolean,
  debounce?: number,
) => invoke<{ job_id: string; files_scanned: number; docs_added: number; errors: number }>('scan_repo', { repoPath, filters, watch, debounce })

export const docsCreate = (repo_id: string, slug: string, title: string, body: string) =>
  invoke<{ doc_id: string }>('docs_create', { payload: { repo_id, slug, title, body } })

export const docsUpdate = (doc_id: string, body: string, message?: string) =>
  invoke<{ version_id: string }>('docs_update', { payload: { doc_id, body, message } })

export const docsGet = (doc_id: string, content?: boolean) => invoke<any>('docs_get', { docId: doc_id, content })

export const docsDelete = (doc_id: string) => invoke<{ deleted: boolean }>('docs_delete', { docId: doc_id })

export const search = (query: string, repo_id?: string, limit = 50, offset = 0) =>
  invoke<SearchHit[]>('search', { repoId: repo_id, query, limit, offset })

export const serveApiStart = (port?: number) => invoke<void>('serve_api_start', { port })

export type GraphDoc = { id: string; slug: string; title: string }
export const graphBacklinks = (doc_id: string) => invoke<GraphDoc[]>('graph_backlinks', { docId: doc_id })
export const graphNeighbors = (doc_id: string, depth = 1) => invoke<GraphDoc[]>('graph_neighbors', { docId: doc_id, depth })

export const aiRun = (provider: string, doc_id: string, prompt: string, anchor_id?: string) =>
  invoke<{ trace_id: string; text: string }>('ai_run', { provider, docId: doc_id, anchorId: anchor_id, prompt })

export const anchorsUpsert = (doc_id: string, anchor_id: string, line: number) =>
  invoke<{ ok: boolean }>('anchors_upsert', { docId: doc_id, anchorId: anchor_id, line })
