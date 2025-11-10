declare module '@tanstack/react-router' {
  // Minimal shim until the plugin exposes typings for file-route helpers in our setup.
  export function createFileRoute<TPath extends string = string>(path: TPath): any
}

