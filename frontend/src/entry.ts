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
  ModuleServiceRegistry,
  SlotRegistry,
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
import ContactsDataCard from './ContactsDataCard'
import ContactPickerDialog from './ContactPickerDialog'
import { pickContact } from './contactPickerStore'
import { registerDataCardRenderer } from './kubunoData'

export const sdkVersion = SDK_VERSION

export function register() {
  FaviconRegistry.register('contacts', '/contacts-logo.svg')

  // Contact picker, mounted globally by the host shell: any module can open it
  // without navigating to /contacts.
  SlotRegistry.register('app-dialogs', 'contacts', ContactPickerDialog)

  // Services contacts offers to OTHER modules (chat…). Published only while
  // contacts is installed+active → consumers degrade gracefully when absent.
  //   pickContact(opts?: { title?: string }): Promise<KubunoDataEnvelope | null>
  ModuleServiceRegistry.publish('contacts', {
    pickContact,
  })

  // `contacts.contact` JSON envelopes ("Copier pour Kubuno" in the contact menu,
  // or the pickContact service): consumer modules (chat, notes…) resolve this
  // card through `core.data-card`. `contacts.person` = legacy type, kept so
  // envelopes copied before the rename still render.
  registerDataCardRenderer('contacts', {
    types: ['contacts.contact', 'contacts.person'],
    Component: ContactsDataCard,
  })

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
