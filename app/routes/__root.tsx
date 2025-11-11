import * as React from 'react'
import { Link, Outlet, createRootRoute } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'

export const Route = createRootRoute({
  component: Root,
})

function Root() {
  const { t } = useTranslation('common')
  return (
    <div className="min-h-screen bg-white text-black">
      <header className="border-b px-4 py-3 flex gap-4 items-center">
        <h1 className="font-semibold">{t('app.title')}</h1>
        <nav className="flex gap-3 text-sm text-gray-700">
          <Link to="/">{t('nav.home')}</Link>
          <Link to="/search">{t('nav.search')}</Link>
          <Link to="/repo">{t('nav.repos')}</Link>
          <Link to="/graph/">{t('nav.graph')}</Link>
          <Link to="/settings/providers">{t('nav.settings')}</Link>
        </nav>
      </header>
      <Outlet />
    </div>
  )
}
