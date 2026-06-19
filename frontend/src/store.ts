import { create } from 'zustand'
import { Contact, Group, Label, contactsApi } from './api'

export type View =
  | 'all' | 'starred' | 'trashed' | 'group' | 'directory' | 'duplicates'
  | 'label' | 'archived' | 'birthdays' | 'reminders' | 'frequent' | 'followup'
  | 'settings'

export type ViewMode = 'list' | 'grid' | 'table'

interface ContactsState {
  contacts:      Contact[]
  total:         number
  groups:        Group[]
  labels:        Label[]
  dueCount:      number
  selectedId:    string | null
  selectedIds:   Set<string>
  lastClickedId: string | null
  view:          View
  viewMode:      ViewMode
  sort:          string
  filter:        string | null
  activeGroupId: string | null
  activeLabelId: string | null
  searchQuery:   string
  isLoading:     boolean
  editorOpen:    boolean
  importOpen:    boolean

  setView:          (view: View, id?: string) => void
  setViewMode:      (m: ViewMode) => void
  setSort:          (s: string) => void
  setFilter:        (f: string | null) => void
  setSelectedId:    (id: string | null) => void
  setSearchQuery:   (q: string) => void
  setEditorOpen:    (v: boolean) => void
  setImportOpen:    (v: boolean) => void

  // Multi-selection
  toggleSelect:     (id: string, shiftKey?: boolean) => void
  selectAll:        () => void
  clearSelection:   () => void

  fetchContacts:    () => Promise<void>
  fetchGroups:      () => Promise<void>
  fetchLabels:      () => Promise<void>
  fetchDueCount:    () => Promise<void>
  updateContact:    (c: Contact) => void
  removeContact:    (id: string) => void
}

export const useContactsStore = create<ContactsState>((set, get) => ({
  contacts:      [],
  total:         0,
  groups:        [],
  labels:        [],
  dueCount:      0,
  selectedId:    null,
  selectedIds:   new Set(),
  lastClickedId: null,
  view:          'all',
  viewMode:      'list',
  sort:          'name',
  filter:        null,
  activeGroupId: null,
  activeLabelId: null,
  searchQuery:   '',
  isLoading:     false,
  editorOpen:    false,
  importOpen:    false,

  setEditorOpen: (v) => set({ editorOpen: v }),
  setImportOpen: (v) => set({ importOpen: v }),
  setViewMode:   (m) => set({ viewMode: m }),

  setSort:   (s) => { set({ sort: s }); get().fetchContacts() },
  setFilter: (f) => { set({ filter: f }); get().fetchContacts() },

  setView: (view, id) => {
    set({
      view,
      activeGroupId: view === 'group' ? (id ?? null) : null,
      activeLabelId: view === 'label' ? (id ?? null) : null,
      selectedId: null,
      selectedIds: new Set(),
    })
    get().fetchContacts()
  },

  setSelectedId: (id) => set({ selectedId: id }),

  setSearchQuery: (q) => {
    set({ searchQuery: q })
    get().fetchContacts()
  },

  toggleSelect: (id, shiftKey) => {
    const { selectedIds, lastClickedId, contacts } = get()
    const next = new Set(selectedIds)
    if (shiftKey && lastClickedId) {
      // Range selection between the last clicked row and this one.
      const ids = contacts.map(c => c.id)
      const a = ids.indexOf(lastClickedId)
      const b = ids.indexOf(id)
      if (a !== -1 && b !== -1) {
        const [lo, hi] = a < b ? [a, b] : [b, a]
        for (let i = lo; i <= hi; i++) next.add(ids[i])
      }
    } else if (next.has(id)) {
      next.delete(id)
    } else {
      next.add(id)
    }
    set({ selectedIds: next, lastClickedId: id })
  },

  selectAll: () => {
    const { contacts, selectedIds } = get()
    if (selectedIds.size === contacts.length) {
      set({ selectedIds: new Set() })
    } else {
      set({ selectedIds: new Set(contacts.map(c => c.id)) })
    }
  },

  clearSelection: () => set({ selectedIds: new Set() }),

  fetchContacts: async () => {
    set({ isLoading: true })
    try {
      const { view, activeGroupId, activeLabelId, searchQuery, sort, filter } = get()
      // Interaction-driven views hit dedicated endpoints.
      if (view === 'frequent' || view === 'followup') {
        const res = view === 'frequent'
          ? await contactsApi.frequent(60)
          : await contactsApi.followUp(90, 60)
        set({ contacts: res.data.contacts, total: res.data.contacts.length })
        return
      }
      const params = {
        q:        searchQuery || undefined,
        starred:  view === 'starred' ? true : undefined,
        trashed:  view === 'trashed' ? true : undefined,
        archived: view === 'archived' ? true : undefined,
        group_id: view === 'group' ? (activeGroupId ?? undefined) : undefined,
        label_id: view === 'label' ? (activeLabelId ?? undefined) : undefined,
        filter:   filter ?? undefined,
        sort,
        limit:    500,
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
    } catch { /* silent */ }
  },

  fetchLabels: async () => {
    try {
      const res = await contactsApi.listLabels()
      set({ labels: res.data.labels })
    } catch { /* silent */ }
  },

  fetchDueCount: async () => {
    try {
      const res = await contactsApi.listReminders(false)
      set({ dueCount: res.data.due_count })
    } catch { /* silent */ }
  },

  updateContact: (c) =>
    set(s => ({ contacts: s.contacts.map(x => x.id === c.id ? c : x) })),

  removeContact: (id) =>
    set(s => ({
      contacts: s.contacts.filter(x => x.id !== id),
      selectedId: s.selectedId === id ? null : s.selectedId,
    })),
}))
