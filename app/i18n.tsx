import React from 'react'
import i18n from 'i18next'
import { I18nextProvider, initReactI18next } from 'react-i18next'

const namespaces = ['common', 'search', 'graph', 'editor', 'settings', 'repo'] as const
type Ns = (typeof namespaces)[number]

async function loadNamespace(lang: string, ns: Ns) {
  const url = `/locales/${lang}/${ns}.json`
  const res = await fetch(url)
  if (!res.ok) return {}
  return (await res.json()) as Record<string, string>
}

async function init(lang = 'en') {
  const entries = await Promise.all(
    namespaces.map(async (ns) => [ns, await loadNamespace(lang, ns)] as const),
  )
  const resources: any = { [lang]: {} }
  for (const [ns, data] of entries) resources[lang][ns] = data
  if (!i18n.isInitialized) {
    await i18n
      .use(initReactI18next)
      .init({
        lng: lang,
        fallbackLng: 'en',
        resources,
        interpolation: { escapeValue: false },
        defaultNS: 'common',
      })
  } else {
    for (const [ns, data] of entries) i18n.addResourceBundle(lang, ns, data, true, true)
  }
  return i18n
}

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [ready, setReady] = React.useState(i18n.isInitialized)
  React.useEffect(() => {
    if (!i18n.isInitialized) init().then(() => setReady(true))
  }, [])
  if (!ready) return null
  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
}

