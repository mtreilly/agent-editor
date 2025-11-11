export interface PluginContext {
  version: 'v1'
  permissions: Record<string, boolean>
  ipc: {
    call<T>(method: string, params?: any): Promise<T>
    on(event: string, handler: (payload: any) => void): () => void
  }
}

export interface Contributions {
  commands?: Array<{ id: string; title: string; run: (args: any) => Promise<void> | void }>
  views?: Array<{ id: string; mount: (el: HTMLElement, ctx: PluginContext) => void }>
  scanners?: Array<{ id: string; globs: string[]; handle: (file: { path: string; content: string }) => Promise<void> }>
  aiPrompts?: Array<{ id: string; title: string; build: (ctx: any) => Promise<string> }>
  renderers?: Array<{ type: 'node' | 'mark'; name: string; render: (node: any) => HTMLElement }>
}

export interface PluginV1 {
  name: string
  version: string
  kind: 'ui' | 'core'
  activate(ctx: PluginContext): Promise<Contributions>
  deactivate?(): Promise<void>
}

