import * as React from 'react'
import { Link, Outlet, createRootRoute } from '@tanstack/react-router'

export const Route = createRootRoute({
  component: Root,
})

function Root() {
  return (
    <div className="min-h-screen bg-white text-black">
      <header className="border-b px-4 py-3 flex gap-4 items-center">
        <h1 className="font-semibold">agent-editor</h1>
        <nav className="flex gap-3 text-sm text-gray-700">
          <Link to="/">Home</Link>
          <Link to="/search">Search</Link>
          <Link to="/repo">Repos</Link>
        </nav>
      </header>
      <Outlet />
    </div>
  )
}
