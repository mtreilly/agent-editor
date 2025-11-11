export type Command = { id: string; title: string; run: () => void | Promise<void> }

let registry: Command[] = []

export function setCommands(cmds: Command[]) {
  registry = cmds
}

export function getCommands(): Command[] {
  return registry
}

export function builtinCommands(navigate: (to: string) => void): Command[] {
  return [
    { id: 'nav.search', title: 'Search', run: () => navigate('/search') },
    { id: 'nav.plugins', title: 'Plugins', run: () => navigate('/plugins') },
    { id: 'nav.settings.providers', title: 'Settings: Providers', run: () => navigate('/settings/providers') },
  ]
}

