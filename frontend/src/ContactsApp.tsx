import { useEffect, useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Upload, MoreHorizontal, UserPlus, Download, X, Search,
  ArrowDownUp, Filter, List, LayoutGrid, Table as TableIcon,
} from 'lucide-react'
import { MenuDropdown, type MenuItem, type MenuDropdownPos } from '@ui'
import { useContactsStore, type ViewMode } from './store'
import ContactAvatar from './ContactAvatar'
import ContactDetail from './ContactDetail'
import ContactEditor from './ContactEditor'
import DirectoryView from './DirectoryView'
import ImportModal from './ImportModal'
import { SelectionBar, RowCheckbox, LabelChips } from './widgets'
import { BirthdaysView, RemindersView, DuplicatesView, SettingsView } from './featureViews'
import { ContactMenuProvider, useContactMenu } from './contactMenu'
import { contactsApi, type Contact } from './api'

function EmptyIllustration() {
  return (
    <svg width="160" height="160" viewBox="0 0 180 180" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M68 128 Q62 150 56 165 L124 165 Q118 150 112 128 Z" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5" strokeLinejoin="round"/>
      <rect x="72" y="118" width="36" height="12" rx="5" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="90" cy="40" r="16" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="90" cy="35" r="6" fill="#1a73e8"/>
      <path d="M79 52 Q90 48 101 52" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
      <line x1="40" y1="165" x2="140" y2="165" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
    </svg>
  )
}

// ── List row ────────────────────────────────────────────────────────────────
function ContactRow({ contact, isSelected, isChecked, onClick }: {
  contact: Contact; isSelected: boolean; isChecked: boolean; onClick: () => void
}) {
  const { t } = useTranslation('contacts')
  const { open } = useContactMenu()
  const email = contact.emails[0]?.value ?? ''
  const phone = contact.phones[0]?.value ?? ''
  return (
    <div
      onClick={onClick}
      onContextMenu={e => open(e, contact)}
      className={`w-full grid items-center px-4 py-2 hover:bg-surface-1 transition-colors group text-left cursor-pointer
                  grid-cols-[36px_40px_1fr] sm:grid-cols-[36px_40px_1.4fr_1.4fr_1fr]
                  ${isSelected ? 'bg-primary-light' : isChecked ? 'bg-primary-light/40' : ''}`}
    >
      <div className={isChecked ? '' : 'opacity-0 group-hover:opacity-100'}><RowCheckbox contact={contact} checked={isChecked} /></div>
      <ContactAvatar contact={contact} size="sm" />
      <span className={`text-sm truncate pr-4 flex items-center gap-2 ${isSelected ? 'text-primary font-medium' : 'text-text-primary'}`}>
        {contact.display_name || t('no_name')}
        <LabelChips labelIds={contact.label_ids} />
      </span>
      <span className="hidden sm:block text-sm text-text-secondary truncate pr-4">{email}</span>
      <span className="hidden sm:block text-sm text-text-secondary truncate">{phone}</span>
    </div>
  )
}

// ── Grid card ───────────────────────────────────────────────────────────────
function ContactCard({ contact, isSelected, isChecked, onClick }: {
  contact: Contact; isSelected: boolean; isChecked: boolean; onClick: () => void
}) {
  const { t } = useTranslation('contacts')
  const { open } = useContactMenu()
  return (
    <div onClick={onClick} onContextMenu={e => open(e, contact)}
      className={`relative rounded-xl border p-4 flex flex-col items-center text-center cursor-pointer transition-colors group
                  ${isSelected || isChecked ? 'border-primary bg-primary-light/40' : 'border-border hover:bg-surface-1'}`}>
      <div className={`absolute top-2 left-2 ${isChecked ? '' : 'opacity-0 group-hover:opacity-100'}`}>
        <RowCheckbox contact={contact} checked={isChecked} />
      </div>
      <ContactAvatar contact={contact} size="xl" />
      <p className="text-sm font-medium text-text-primary mt-2 truncate w-full">{contact.display_name || t('no_name')}</p>
      {contact.organization && <p className="text-xs text-text-secondary truncate w-full">{contact.organization}</p>}
      <p className="text-xs text-text-secondary truncate w-full mt-0.5">{contact.emails[0]?.value ?? ''}</p>
      <div className="mt-1"><LabelChips labelIds={contact.label_ids} /></div>
    </div>
  )
}

export default function ContactsApp() {
  return <ContactMenuProvider><ContactsAppInner /></ContactMenuProvider>
}

// Pilotage responsive en JS (matchMedia) car les variantes `sm:` d'un module qui
// annulent une classe de base (w-full→sm:w-56…) sont écrasées par l'utilitaire de
// base du host (couche utilities > kubuno-module).
function useIsMobile(): boolean {
  const [m, setM] = useState(() =>
    typeof window !== 'undefined' && window.matchMedia('(max-width: 1023px)').matches)
  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)')
    const on = () => setM(mq.matches)
    mq.addEventListener('change', on)
    return () => mq.removeEventListener('change', on)
  }, [])
  return m
}

function ContactsAppInner() {
  const isMobile = useIsMobile()
  const { t } = useTranslation('contacts')
  const { open: openContactMenu } = useContactMenu()
  const s = useContactsStore()
  const {
    contacts, total, selectedId, selectedIds, view, viewMode, sort,
    fetchContacts, fetchGroups, fetchLabels, fetchDueCount,
    setSelectedId, toggleSelect, setViewMode, setSort, setFilter, setSearchQuery,
    editorOpen, setEditorOpen, importOpen, setImportOpen,
  } = s

  const [searchLocal, setSearchLocal] = useState('')
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)
  const debRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => { fetchContacts(); fetchGroups(); fetchLabels(); fetchDueCount() }, [])

  function onSearchChange(v: string) {
    setSearchLocal(v)
    clearTimeout(debRef.current)
    debRef.current = setTimeout(() => setSearchQuery(v), 250)
  }

  const selected = contacts.find(c => c.id === selectedId) ?? null

  // Routed full-area views
  if (editorOpen) return <div className="flex-1 overflow-hidden"><ContactEditor onDone={() => { setEditorOpen(false); fetchContacts() }} /></div>
  if (view === 'directory') return <div className="flex-1 overflow-hidden"><DirectoryView /></div>
  if (view === 'birthdays') return <div className="flex-1 overflow-hidden flex"><BirthdaysView />{selected && <DetailPanel contact={selected} onClose={() => setSelectedId(null)} />}</div>
  if (view === 'reminders') return <div className="flex-1 overflow-hidden flex"><RemindersView />{selected && <DetailPanel contact={selected} onClose={() => setSelectedId(null)} />}</div>
  if (view === 'duplicates') return <div className="flex-1 overflow-hidden"><DuplicatesView /></div>
  if (view === 'settings') return <div className="flex-1 overflow-hidden"><SettingsView /></div>

  const viewLabel: Record<string, string> = {
    all: t('title_all'), starred: t('title_starred'), trashed: t('title_trashed'),
    group: t('title_group'), label: t('labels_section'), archived: t('nav_archived'),
    frequent: t('nav_frequent'), followup: t('nav_followup'),
  }
  const title = viewLabel[view] ?? t('title_all')
  const isEmpty = !contacts.length

  // Alphabetical grouping for the list view
  const grouped = new Map<string, Contact[]>()
  for (const c of contacts) {
    const first = c.display_name?.[0]?.toUpperCase() ?? '#'
    const letter = /[A-Z]/.test(first) ? first : '#'
    if (!grouped.has(letter)) grouped.set(letter, [])
    grouped.get(letter)!.push(c)
  }
  const sortedKeys = [...grouped.keys()].sort((a, b) => a === '#' ? 1 : b === '#' ? -1 : a.localeCompare(b))

  function openSortMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const opt = (key: string, label: string): MenuItem => ({ type: 'action', label, checked: sort === key, onClick: () => setSort(key) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: [
      opt('name', t('sort_name')), opt('name_desc', t('sort_name_desc')),
      opt('recent', t('sort_recent')), opt('updated', t('sort_updated')),
      opt('organization', t('sort_org')), opt('last_interaction', t('sort_interaction')),
    ]})
  }
  function openFilterMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const opt = (key: string | null, label: string): MenuItem => ({ type: 'action', label, checked: s.filter === key, onClick: () => setFilter(key) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: [
      opt(null, t('filter_all')), { type: 'separator' },
      opt('incomplete', t('filter_incomplete')),
      opt('missing_email', t('filter_no_email')), opt('missing_phone', t('filter_no_phone')),
      opt('no_group', t('filter_no_group')), opt('no_label', t('filter_no_label')),
    ]})
  }
  function openActionsMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setMenu({ pos: { top: r.bottom + 4, left: r.left - 120 }, items: [
      { type: 'action', label: t('set_export_vcf'), icon: <Download size={15} />, onClick: () => contactsApi.exportVcf({}) },
      { type: 'action', label: t('set_export_csv'), icon: <Download size={15} />, onClick: () => contactsApi.exportCsv({}) },
      { type: 'action', label: t('import'), icon: <Upload size={15} />, onClick: () => setImportOpen(true) },
    ]})
  }

  const ToolBtn = ({ icon, label, onClick, active }: { icon: React.ReactNode; label: string; onClick: (e: React.MouseEvent) => void; active?: boolean }) => (
    <button onClick={onClick} title={label}
      className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-sm transition-colors ${active ? 'bg-primary-light text-primary' : 'hover:bg-surface-2 text-text-secondary'}`}>
      {icon}<span className="hidden lg:inline">{label}</span>
    </button>
  )
  const modeBtn = (m: ViewMode, icon: React.ReactNode) => (
    <button onClick={() => setViewMode(m)} title={m}
      className={`p-1.5 rounded-lg transition-colors ${viewMode === m ? 'bg-primary-light text-primary' : 'hover:bg-surface-2 text-text-secondary'}`}>{icon}</button>
  )

  return (
    <div className="flex-1 flex overflow-hidden">
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <div className={`${isMobile ? 'px-4' : 'px-6'} pt-4 pb-2 flex items-center flex-wrap gap-x-3 gap-y-2 flex-shrink-0`}>
          <h1 className="text-2xl font-normal text-text-primary truncate">
            {title}{total > 0 && <span className="text-text-secondary ml-2 text-xl">({total})</span>}
          </h1>
          <div className="flex-1" />
          {/* Search — pleine largeur sur mobile (passe à la ligne), fixe ensuite */}
          <div className={`relative no-print ${isMobile ? 'w-full' : 'w-auto'}`}>
            <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary" />
            <input value={searchLocal} onChange={e => onSearchChange(e.target.value)}
              placeholder={t('contacts_search_ph')} title={t('search_help')}
              className={`pl-9 pr-3 py-2 ${isMobile ? 'w-full' : 'w-56'} rounded-full bg-surface-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/30`} />
          </div>
        </div>

        {/* Toolbar */}
        <div className="px-6 pb-1 flex items-center gap-1 flex-shrink-0 no-print">
          <ToolBtn icon={<ArrowDownUp size={15} />} label={t('sort_by')} onClick={openSortMenu} />
          <ToolBtn icon={<Filter size={15} />} label={t('filter_label')} onClick={openFilterMenu} active={!!s.filter} />
          <div className="flex-1" />
          <div className="flex items-center gap-0.5 bg-surface-1 rounded-lg p-0.5">
            {modeBtn('list', <List size={16} />)}
            {modeBtn('grid', <LayoutGrid size={16} />)}
            {modeBtn('table', <TableIcon size={16} />)}
          </div>
          <button onClick={openActionsMenu} className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary"><MoreHorizontal size={16} /></button>
        </div>

        <SelectionBar />

        {isEmpty ? (
          <div className="flex-1 flex flex-col items-center justify-center gap-4">
            <EmptyIllustration />
            <p className="text-base text-text-primary">{t('empty')}</p>
            <div className="flex items-center gap-8">
              <button onClick={() => setEditorOpen(true)} className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"><UserPlus size={16} />{t('create_contact')}</button>
              <button onClick={() => setImportOpen(true)} className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"><Download size={16} />{t('import_contacts')}</button>
            </div>
          </div>
        ) : viewMode === 'grid' ? (
          <div className="flex-1 overflow-y-auto px-4 pb-6">
            <div className="grid gap-3" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))' }}>
              {contacts.map(c => (
                <ContactCard key={c.id} contact={c} isSelected={c.id === selectedId} isChecked={selectedIds.has(c.id)}
                  onClick={() => setSelectedId(c.id === selectedId ? null : c.id)} />
              ))}
            </div>
          </div>
        ) : viewMode === 'table' ? (
          <div className="flex-1 overflow-auto">
            <table className="w-full text-sm">
              <thead className="sticky top-0 bg-white border-b border-border">
                <tr className="text-left text-text-secondary">
                  <th className="px-4 py-2 font-medium">{t('col_name')}</th>
                  <th className="px-4 py-2 font-medium">{t('col_email')}</th>
                  <th className={`${isMobile ? 'hidden' : ''} px-4 py-2 font-medium`}>{t('col_phone')}</th>
                  <th className={`${isMobile ? 'hidden' : ''} px-4 py-2 font-medium`}>{t('sort_org')}</th>
                </tr>
              </thead>
              <tbody>
                {contacts.map(c => (
                  <tr key={c.id} onClick={() => setSelectedId(c.id === selectedId ? null : c.id)}
                    onContextMenu={e => openContactMenu(e, c)}
                    className={`cursor-pointer hover:bg-surface-1 border-b border-border/50 ${c.id === selectedId ? 'bg-primary-light' : selectedIds.has(c.id) ? 'bg-primary-light/40' : ''}`}>
                    <td className="px-4 py-2 flex items-center gap-2"><RowCheckbox contact={c} checked={selectedIds.has(c.id)} />{c.display_name || t('no_name')}</td>
                    <td className="px-4 py-2 text-text-secondary">{c.emails[0]?.value ?? ''}</td>
                    <td className={`${isMobile ? 'hidden' : ''} px-4 py-2 text-text-secondary`}>{c.phones[0]?.value ?? ''}</td>
                    <td className={`${isMobile ? 'hidden' : ''} px-4 py-2 text-text-secondary`}>{c.organization ?? ''}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto">
            <div className="grid px-4 py-2 border-b border-border sticky top-0 bg-white z-10 grid-cols-[36px_40px_1fr] sm:grid-cols-[36px_40px_1.4fr_1.4fr_1fr]">
              <div /><div />
              <span className="text-sm font-medium text-text-secondary">{t('col_name')}</span>
              <span className="hidden sm:block text-sm font-medium text-text-secondary">{t('col_email')}</span>
              <span className="hidden sm:block text-sm font-medium text-text-secondary">{t('col_phone')}</span>
            </div>
            {sortedKeys.map(letter => (
              <div key={letter}>
                <div className="px-4 py-1.5 text-xs font-semibold text-text-secondary">{letter}</div>
                {grouped.get(letter)!.map(c => (
                  <ContactRow key={c.id} contact={c} isSelected={c.id === selectedId} isChecked={selectedIds.has(c.id)}
                    onClick={() => setSelectedId(c.id === selectedId ? null : c.id)} />
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      {selected && <DetailPanel contact={selected} onClose={() => setSelectedId(null)} />}
      {importOpen && <ImportModal onClose={() => setImportOpen(false)} />}
      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
    </div>
  )
}

function DetailPanel({ contact, onClose }: { contact: Contact; onClose: () => void }) {
  const isMobile = useIsMobile()
  return (
    // Plein écran (overlay) sur mobile ; panneau latéral 380px sur desktop.
    // matchMedia (pas `md:`) car `fixed`/`w-full` du host écrasent les variantes
    // responsives du module (couche utilities > kubuno-module).
    <div className={isMobile
      ? 'fixed inset-0 z-30 bg-white overflow-y-auto'
      : 'flex-shrink-0 w-[380px] border-l border-border bg-white overflow-y-auto'}>
      <div className="relative">
        <button onClick={onClose} className="absolute top-3 right-3 w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary z-10"><X size={16} /></button>
        <ContactDetail contact={contact} />
      </div>
    </div>
  )
}
