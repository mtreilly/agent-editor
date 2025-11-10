import * as React from 'react'
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  component: Index,
})

function Index() {
  return (
    <main className="p-6">
      <h1 className="text-2xl font-bold">agent-editor</h1>
      <p className="text-sm text-gray-600">TanStack Start (client-only) + Tailwind v4</p>
    </main>
  )
}
