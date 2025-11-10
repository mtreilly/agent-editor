declare module '@tanstack/react-router' {
  // Augment to allow path overload in file-route helpers for our setup
  export function createFileRoute<TPath extends string = string>(path: TPath): any
}

