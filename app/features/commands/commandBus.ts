export type Command = { id: string; title: string; run: () => void | Promise<void> }

let baseRegistry: Command[] = []
const extraRegistries = new Map<string, Command[]>()

export function setCommands(cmds: Command[]) {
  baseRegistry = cmds
}

export function registerCommands(owner: string, cmds: Command[]) {
  extraRegistries.set(owner, cmds)
}

export function unregisterCommands(owner: string) {
  extraRegistries.delete(owner)
}

export function getCommands(): Command[] {
  const extras: Command[] = []
  for (const arr of extraRegistries.values()) extras.push(...arr)
  return [...baseRegistry, ...extras]
}

export function builtinCommands(navigate: (to: string) => void): Command[] {
  return [
    { id: 'nav.search', title: 'Search', run: () => navigate('/search') },
    { id: 'nav.plugins', title: 'Plugins', run: () => navigate('/plugins') },
    { id: 'nav.settings.providers', title: 'Settings: Providers', run: () => navigate('/settings/providers') },
  ]
}
