/**
 * Point d'entrée du bundle MODULE contacts, chargé à l'exécution. Buildé
 * séparément via `vite.module.config.ts` ; specifiers partagés résolus au runtime
 * par l'import map du host. Le host importe ce fichier puis appelle `register()` ;
 * `sdkVersion` permet de rejeter une incompatibilité de contrat.
 */
import { lazy } from 'react'
import { Users as ContactsIcon, Star, Building2 } from 'lucide-react'
import {
  RouteRegistry,
  WaffleAppRegistry,
  FaviconRegistry,
  ModuleSettingsRegistry,
  ExtensionRegistry,
  ModuleServiceRegistry,
  SlotRegistry,
  registerMentionProvider,
  useSidebarStore,
  useSearchStore,
  useToolbarStore,
  useRightPanelStore,
  SDK_VERSION,
} from '@kubuno/sdk'
import './index.css'
import './i18n'
import { useContactsStore } from './store'
import { contactsApi } from './api'
import ContactsLogo from './ContactsLogo'
import ContactsSidebarBody from './ContactsSidebarBody'
import ContactsMiniPanel from './ContactsMiniPanel'
import ContactsDataCard from './ContactsDataCard'
import ContactPickerDialog from './ContactPickerDialog'
import { pickContact } from './contactPickerStore'
import { registerDataCardRenderer } from './kubunoData'
import { registerContactsAdmin } from './admin/ContactsAdminPanel'

export const sdkVersion = SDK_VERSION

export function register() {
  FaviconRegistry.register('contacts', '/contacts-logo.png')

  // Contact picker, mounted globally by the host shell: any module can open it
  // without navigating to /contacts.
  SlotRegistry.register('app-dialogs', 'contacts', ContactPickerDialog)

  // Services contacts offers to OTHER modules (chat…). Published only while
  // contacts is installed+active → consumers degrade gracefully when absent.
  //   pickContact(opts?: { title?: string }): Promise<KubunoDataEnvelope | null>
  ModuleServiceRegistry.publish('contacts', {
    pickContact,
  })

  // @mention provider: any @ui field with `mentions` enabled discovers this via
  // the shared 'mentions.provider' extension point and suggests contacts as the
  // user types « @… ». Registered only while contacts is installed+active, so a
  // field degrades to plain text when contacts is absent. Reuses the same list
  // endpoint the mail recipient autocomplete already relies on.
  registerMentionProvider('contacts', {
    id:      'contacts',
    trigger: '@',
    async search(query, opts) {
      const { data } = await contactsApi.listContacts({
        q: query, limit: opts?.limit ?? 6, filter: 'has_email',
      })
      return data.contacts.map(c => {
        const email = c.emails?.[0]?.value
        return {
          id:           c.id,
          label:        c.display_name,
          secondary:    email,
          avatarUrl:    c.avatar_path ? contactsApi.avatarUrl(c.id) : undefined,
          email,
          kubunoUserId: c.kubuno_user_id ?? undefined,
        }
      })
    },
  })

  // What this module knows about a PERSON, offered to whoever shows one — a
  // guest's card in the calendar today. The same shared-point bargain as the
  // mentions above: a generic key, no consumer named, and an instance without
  // contacts simply shows what the directory knows.
  //
  // Asked by address, because that is the one identifier every source shares.
  // ⚠️ `register(point, moduleId, entry)` — le point d'abord.
  ExtensionRegistry.register('person.details', 'contacts', {
    async lookup(person: { email?: string; userId?: string }) {
      // By address when there is one, by ACCOUNT otherwise: an instance whose
      // directory keeps addresses private hands the picker an id and nothing
      // else, and a source keyed on the address alone would find nobody.
      const needle = person.email?.trim().toLowerCase()
      const { data } = await contactsApi.listContacts({
        q: needle ?? '', limit: needle ? 5 : 200, filter: 'has_email',
      })
      const c = needle
        ? data.contacts.find(x => (x.emails ?? []).some(e => e.value?.trim().toLowerCase() === needle))
        : data.contacts.find(x => x.kubuno_user_id === person.userId)
      if (!c) return {}

      const details: Array<{ kind: string; value: string; label?: string }> = []
      const add = (kind: string, value?: string | null, label?: string) => {
        if (value && value.trim()) details.push({ kind, value: value.trim(), label })
      }
      add('job_title',    c.job_title)
      add('organisation', c.organization)
      add('department',   c.department)
      for (const p of (c.phones ?? []).slice(0, 2)) add('phone', p.value, p.label ?? undefined)
      const a = (c.addresses ?? [])[0]
      if (a) add('address', [a.street, a.city, a.country].filter(Boolean).join(', '))

      // A named address elsewhere — a site, a profile page.
      const links = (c.urls ?? [])
        .filter(u => u.value?.trim())
        .slice(0, 2)
        .map(u => ({ label: u.label?.trim() || u.value.trim(), url: u.value.trim() }))

      return {
        // The photo this module holds for them — chosen for this person, so it
        // outranks the one derived from an account id.
        avatar: c.avatar_path ? contactsApi.avatarUrl(c.id) : undefined,
        details,
        links,
        actions: [{
          id: 'contacts.open',
          label: 'Ouvrir la vue détaillée',
          icon: 'open',
          primary: true,
          run: () => { window.location.assign(`/contacts?contact=${c.id}`) },
        }],
      }
    },
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

  // Instance administration (console ▸ Modules ▸ Contacts): the account-directory
  // notice and the shared address book.
  registerContactsAdmin()

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
    // Bottom nav (portrait) / left rail (landscape) on mobile: the primary
    // destinations, each a real route so the shell's NavLink can light the
    // active tab (a hash cannot — every hash shares the /contacts pathname). The
    // secondary views (smart views, labels, groups, trash…) stay in the drawer.
    mobileTabs: [
      { id: 'all',       labelKey: 'contacts:title_all',       Icon: ContactsIcon, path: '/contacts',           end: true },
      { id: 'starred',   labelKey: 'contacts:title_starred',   Icon: Star,         path: '/contacts/starred' },
      { id: 'directory', labelKey: 'contacts:title_directory', Icon: Building2,    path: '/contacts/directory' },
    ],
  })

  useSearchStore.getState().register({
    moduleId:    'contacts',
    routePrefix: '/contacts',
    placeholder: 'Rechercher dans les contacts…',
    placeholderKey: 'contacts:contacts_search_ph',
    onSearch:    (q) => useContactsStore.getState().setSearchQuery(q),
  })

  // Side panel: look a contact up without leaving whatever you are writing.
  useRightPanelStore.getState().registerEntry({
    moduleId:       'contacts',
    icon:           ContactsLogo,
    label:          'Contacts',
    panelComponent: ContactsMiniPanel,
    openPath:       '/contacts',
  })

  // Bare toolbar on the settings page (no module toolbar there).
  useToolbarStore.getState().register({
    moduleId:    'contacts-settings',
    routePrefix: '/contacts/settings',
  })

  // Routes
  const ContactsApp          = lazy(() => import('./ContactsApp'))
  const ContactsSettingsPage = lazy(() => import('./ContactsSettingsPage'))
  const ContactDetailPage    = lazy(() => import('./detail/ContactDetailPage'))

  RouteRegistry.register('contacts',          ContactsApp)
  RouteRegistry.register('contacts/starred',  ContactsApp)
  RouteRegistry.register('contacts/trashed',  ContactsApp)
  RouteRegistry.register('contacts/directory', ContactsApp)
  // A contact has its own address, like Google's /person/<id>: the card is a
  // destination, not a strip beside the list.
  RouteRegistry.register('contacts/person/:id', ContactDetailPage)
  RouteRegistry.register('contacts/settings', ContactsSettingsPage)
}
