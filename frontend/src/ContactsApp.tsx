import { useEffect, useState, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useContactsStore } from './store'
import { useIsMobile } from './hooks/useIsMobile'
import { useContactsRoute } from './hooks/useContactsRoute'
import ContactEditor from './editor/ContactEditor'
import DirectoryView from './DirectoryView'
import ImportModal from './ImportModal'
import ListHeader from './list/ListHeader'
import ListToolbar from './list/ListToolbar'
import ContactListView from './list/ContactListView'
import EmptyState from './list/EmptyState'
import { SelectionBar } from './widgets'
import { BirthdaysView, RemindersView, DuplicatesView, SettingsView } from './featureViews'
import OtherContactsView from './views/OtherContactsView'
import UnitMembersView from './views/UnitMembersView'
import { ContactMenuProvider, useContactMenu } from './contactMenu'
import { CONTACTS_BASE } from './hashRoute'

export default function ContactsApp() {
  return <ContactMenuProvider><ContactsAppInner /></ContactMenuProvider>
}

function ContactsAppInner() {
  const isMobile = useIsMobile()
  const { t } = useTranslation('contacts')
  const { overlayOpen } = useContactMenu()
  const navigate = useNavigate()
  useContactsRoute()

  const s = useContactsStore()
  const {
    contacts, total, selectedId, selectedIds, view, viewMode,
    fetchContacts, fetchGroups, fetchLabels, fetchDueCount,
    setSelectedId, setSearchQuery,
    editorOpen, setEditorOpen, importOpen, setImportOpen, trashContacts,
  } = s

  const [searchLocal, setSearchLocal] = useState('')
  const debRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => { fetchContacts(); fetchGroups(); fetchLabels(); fetchDueCount() }, [])

  // Deep link `?contact=<id>`, still emitted by cross-module data cards: send it
  // to the contact's own page, which is now a real address. Replacing the entry
  // keeps the query string out of the history.
  useEffect(() => {
    const cid = new URLSearchParams(window.location.search).get('contact')
    if (cid) navigate(`${CONTACTS_BASE}/person/${cid}`, { replace: true })
  }, [navigate])

  // Delete key → the "Supprimer" context-menu entry (store.trashContacts). Only
  // on the views that render the list; the trash view's permanent delete stays a
  // deliberate click.
  useEffect(() => {
    const listViews = ['all', 'starred', 'group', 'label', 'archived', 'frequent', 'followup']
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Delete') return
      const el = e.target as HTMLElement | null
      if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable)) return
      if (!listViews.includes(view) || editorOpen || importOpen || overlayOpen) return
      const targets = selectedIds.size ? [...selectedIds] : selectedId ? [selectedId] : []
      if (!targets.length) return
      e.preventDefault()
      trashContacts(targets).catch(() => {})
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [view, editorOpen, importOpen, overlayOpen, selectedIds, selectedId, trashContacts])

  function onSearchChange(v: string) {
    setSearchLocal(v)
    clearTimeout(debRef.current)
    debRef.current = setTimeout(() => setSearchQuery(v), 250)
  }

  // Routed full-area views.
  if (editorOpen) return <div className="flex-1 overflow-hidden"><ContactEditor onDone={() => { setEditorOpen(false); fetchContacts() }} /></div>
  if (view === 'directory') return <div className="flex-1 overflow-hidden"><DirectoryView /></div>
  if (view === 'birthdays') return <div className="flex-1 overflow-hidden"><BirthdaysView /></div>
  if (view === 'reminders') return <div className="flex-1 overflow-hidden"><RemindersView /></div>
  if (view === 'other')      return <div className="flex-1 overflow-hidden"><OtherContactsView /></div>
  if (view === 'unit')       return <div className="flex-1 overflow-hidden"><UnitMembersView /></div>
  if (view === 'duplicates') return <div className="flex-1 overflow-hidden"><DuplicatesView /></div>
  if (view === 'settings') return <div className="flex-1 overflow-hidden"><SettingsView /></div>

  const viewLabel: Record<string, string> = {
    all: t('title_all'), starred: t('title_starred'), trashed: t('title_trashed'),
    group: t('title_group'), label: t('labels_section'), archived: t('nav_archived'),
    frequent: t('nav_frequent'), followup: t('nav_followup'),
  }
  const title = viewLabel[view] ?? t('title_all')
  const isEmpty = !contacts.length

  return (
    <div className="flex-1 flex overflow-hidden">
      <div className="flex-1 flex flex-col overflow-hidden">
        <ListHeader title={title} total={total} search={searchLocal} onSearch={onSearchChange} isMobile={isMobile} onCreate={() => setEditorOpen(true)} />
        <ListToolbar isMobile={isMobile} />
        <SelectionBar />

        {isEmpty ? (
          <EmptyState onCreate={() => setEditorOpen(true)} onImport={() => setImportOpen(true)} />
        ) : (
          <ContactListView
            contacts={contacts} viewMode={viewMode} selectedId={selectedId} selectedIds={selectedIds}
            isMobile={isMobile}
            onSelect={id => navigate(`${CONTACTS_BASE}/person/${id}`)}
          />
        )}
      </div>

      {importOpen && <ImportModal onClose={() => setImportOpen(false)} />}
    </div>
  )
}
