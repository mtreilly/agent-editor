import React from 'react'
import { createRoot } from 'react-dom/client'
import { createRouter, RouterProvider } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import './app.css'
import { I18nProvider } from './i18n'

const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

const rootEl = document.getElementById('root') as HTMLElement
createRoot(rootEl).render(
  <I18nProvider>
    <RouterProvider router={router} />
  </I18nProvider>,
)
