/**
 * Point d'entrée du bundle MODULE contacts, chargé à l'exécution. Buildé
 * séparément via `vite.module.config.ts` ; specifiers partagés résolus au runtime
 * par l'import map du host. Le host importe ce fichier puis appelle `register()` ;
 * `sdkVersion` permet de rejeter une incompatibilité de contrat.
 */
import { lazy } from 'react'
import {
  RouteRegistry,
  WaffleAppRegistry,
  FaviconRegistry,
  useSidebarStore,
  useSearchStore,
  useToolbarStore,
  SDK_VERSION,
} from '@kubuno/sdk'
import './index.css'
import './i18n'
import { useContactsStore } from './store'
import ContactsLogo from './ContactsLogo'
import ContactsSidebarBody from './ContactsSidebarBody'

export const sdkVersion = SDK_VERSION

export function register() {
  FaviconRegistry.register('contacts', '/contacts-logo.svg')

  WaffleAppRegistry.register('contacts', 'Contacts', [
    { id: 'contacts', label: 'Contacts', Icon: ContactsLogo, path: '/contacts' },
  ])

  useToolbarStore.getState().register({
    moduleId:    'contacts',
    routePrefix: '/contacts',
    noPadding:   true,
  })

  useSidebarStore.getState().register({
    moduleId:    'contacts',
    routePrefix: '/contacts',
    SidebarBody: ContactsSidebarBody,
    collapsedBody: true,
  })

  useSearchStore.getState().register({
    moduleId:    'contacts',
    routePrefix: '/contacts',
    placeholder: 'Rechercher dans les contacts…',
    placeholderKey: 'contacts:contacts_search_ph',
    onSearch:    (q) => useContactsStore.getState().setSearchQuery(q),
  })

  // Routes
  const ContactsApp = lazy(() => import('./ContactsApp'))

  RouteRegistry.register('contacts',         ContactsApp)
  RouteRegistry.register('contacts/starred', ContactsApp)
  RouteRegistry.register('contacts/trashed', ContactsApp)
}
