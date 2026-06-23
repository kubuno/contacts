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
  ModuleSettingsRegistry,
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

  // The header gear button opens the per-user Contacts settings while in /contacts.
  ModuleSettingsRegistry.register('contacts')

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

  // Bare toolbar on the settings page (no module toolbar there).
  useToolbarStore.getState().register({
    moduleId:    'contacts-settings',
    routePrefix: '/contacts/settings',
  })

  // Routes
  const ContactsApp          = lazy(() => import('./ContactsApp'))
  const ContactsSettingsPage = lazy(() => import('./ContactsSettingsPage'))

  RouteRegistry.register('contacts',         ContactsApp)
  RouteRegistry.register('contacts/starred', ContactsApp)
  RouteRegistry.register('contacts/trashed', ContactsApp)
  RouteRegistry.register('contacts/settings', ContactsSettingsPage)
}
