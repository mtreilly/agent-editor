# Routing — agent-editor (TanStack Start)

This app uses TanStack Router (Start-style) with file-based routes under `app/routes/`.

## Anatomy of a route
```ts
// app/routes/search.tsx
import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/search')({
  component: SearchPage,
})

function SearchPage() {
  return <main className="p-6">Search</main>
}
```

## Dynamic params
- Use `$` in filenames (e.g., `doc.$id.tsx`).
- Access params via `Route.useParams()`.

```ts
export const Route = createFileRoute('/doc/$id')({ component: DocPage })
function DocPage() {
  const { id } = Route.useParams()
  // fetch or render by id
}
```

## Navigation
- Use the `useNavigate` hook: `navigate({ to: '/search' })`.
- Prefer IDs in URLs (e.g., `/doc/<id>`). For slugs, support both id or slug server-side.

## i18n & a11y
- Extract all visible strings to `public/locales/en/*.json`.
- Use semantic elements (`<main>`, `<section>`, headings), and ARIA attributes for widgets (e.g., listbox, options).
- Use focus management for overlays (e.g., command palette) and keyboard controls (ArrowUp/Down, Enter, Esc).

## Code splitting
- Lazy-load heavy components: `React.lazy(() => import('../features/...'))`.
- Wrap with `React.Suspense` and an accessible fallback.

## Route tree generation
- Start builds `routeTree.gen.ts` automatically in dev; ensure this file is committed or excluded appropriately depending on your workflow. Keep route files colocated and simple.

## Adding a new page (checklist)
1) Create a route file `app/routes/<name>.tsx` with `createFileRoute` and a component.
2) Add i18n keys for visible strings.
3) Validate keyboard navigation and ARIA for interactive widgets.
4) Add an E2E test if the route is user-facing.
