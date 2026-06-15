import { create } from 'zustand'
import { Contact, Group, contactsApi } from './api'

type View = 'all' | 'starred' | 'trashed' | 'group' | 'directory' | 'duplicates'

interface ContactsState {
  contacts:     Contact[]
  total:        number
  groups:       Group[]
  selectedId:   string | null
  view:         View
  activeGroupId: string | null
  searchQuery:  string
  isLoading:    boolean
  editorOpen:   boolean
  importOpen:   boolean

  setView:          (view: View, groupId?: string) => void
  setSelectedId:    (id: string | null) => void
  setSearchQuery:   (q: string) => void
  setEditorOpen:    (v: boolean) => void
  setImportOpen:    (v: boolean) => void
  fetchContacts:    () => Promise<void>
  fetchGroups:      () => Promise<void>
  updateContact:    (c: Contact) => void
  removeContact:    (id: string) => void
}

export const useContactsStore = create<ContactsState>((set, get) => ({
  contacts:      [],
  total:         0,
  groups:        [],
  selectedId:    null,
  view:          'all',
  activeGroupId: null,
  searchQuery:   '',
  isLoading:     false,
  editorOpen:    false,
  importOpen:    false,

  setEditorOpen: (v) => set({ editorOpen: v }),
  setImportOpen: (v) => set({ importOpen: v }),

  setView: (view, groupId) => {
    set({ view, activeGroupId: groupId ?? null, selectedId: null })
    get().fetchContacts()
  },

  setSelectedId: (id) => set({ selectedId: id }),

  setSearchQuery: (q) => {
    set({ searchQuery: q })
    get().fetchContacts()
  },

  fetchContacts: async () => {
    set({ isLoading: true })
    try {
      const { view, activeGroupId, searchQuery } = get()
      const params = {
        q:        searchQuery || undefined,
        starred:  view === 'starred' ? true : undefined,
        trashed:  view === 'trashed' ? true : undefined,
        group_id: view === 'group' ? (activeGroupId ?? undefined) : undefined,
        limit:    200,
      }
      const res = await contactsApi.listContacts(params)
      set({ contacts: res.data.contacts, total: res.data.total })
    } catch {
      // silent
    } finally {
      set({ isLoading: false })
    }
  },

  fetchGroups: async () => {
    try {
      const res = await contactsApi.listGroups()
      set({ groups: res.data.groups })
    } catch {
      // silent
    }
  },

  updateContact: (c) =>
    set(s => ({ contacts: s.contacts.map(x => x.id === c.id ? c : x) })),

  removeContact: (id) =>
    set(s => ({ contacts: s.contacts.filter(x => x.id !== id), selectedId: s.selectedId === id ? null : s.selectedId })),
}))
