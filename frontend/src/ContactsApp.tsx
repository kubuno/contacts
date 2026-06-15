import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Printer, Upload, MoreHorizontal, UserPlus, Download, X } from 'lucide-react'
import { useContactsStore } from './store'
import ContactAvatar from './ContactAvatar'
import ContactDetail from './ContactDetail'
import ContactEditor from './ContactEditor'
import DirectoryView from './DirectoryView'
import ImportModal from './ImportModal'
import type { Contact } from './api'

// ── Empty state illustration ───────────────────────────────────────────────────

function EmptyIllustration() {
  return (
    <svg width="180" height="180" viewBox="0 0 180 180" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Vase */}
      <path d="M68 128 Q62 150 56 165 L124 165 Q118 150 112 128 Z" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5" strokeLinejoin="round"/>
      <rect x="72" y="118" width="36" height="12" rx="5" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      {/* Stems */}
      <path d="M84 118 Q78 92 72 62" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
      <path d="M90 118 L90 54" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
      <path d="M96 118 Q102 92 108 62" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
      {/* Leaves */}
      <ellipse cx="66" cy="84" rx="12" ry="5" transform="rotate(-35 66 84)" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.2"/>
      <ellipse cx="114" cy="84" rx="12" ry="5" transform="rotate(35 114 84)" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.2"/>
      {/* Person left */}
      <circle cx="72" cy="50" r="14" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="72" cy="46" r="5" fill="#1a73e8"/>
      <path d="M63 61 Q72 57 81 61" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
      {/* Person center */}
      <circle cx="90" cy="40" r="16" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="90" cy="35" r="6" fill="#1a73e8"/>
      <path d="M79 52 Q90 48 101 52" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
      {/* Person right */}
      <circle cx="108" cy="50" r="14" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="108" cy="46" r="5" fill="#1a73e8"/>
      <path d="M99 61 Q108 57 117 61" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
      {/* Ground */}
      <line x1="40" y1="165" x2="140" y2="165" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
    </svg>
  )
}

// ── Table row ─────────────────────────────────────────────────────────────────

function ContactRow({ contact, isSelected, onClick }: {
  contact:    Contact
  isSelected: boolean
  onClick:    () => void
}) {
  const { t } = useTranslation('contacts')
  const email = contact.emails[0]?.value ?? ''
  const phone = contact.phones[0]?.value ?? ''

  return (
    <button
      onClick={onClick}
      className={`w-full grid items-center px-4 py-2 hover:bg-surface-1 transition-colors group text-left
                  ${isSelected ? 'bg-primary-light' : ''}`}
      style={{ gridTemplateColumns: '48px 1fr 1fr 1fr' }}
    >
      <ContactAvatar contact={contact} size="sm" />
      <span className={`text-sm truncate pr-4 ${isSelected ? 'text-primary font-medium' : 'text-text-primary'}`}>
        {contact.display_name || t('no_name')}
      </span>
      <span className="text-sm text-text-secondary truncate pr-4">{email}</span>
      <span className="text-sm text-text-secondary truncate">{phone}</span>
    </button>
  )
}

// ── Main component ────────────────────────────────────────────────────────────

export default function ContactsApp() {
  const { t } = useTranslation('contacts')
  const {
    contacts, total, selectedId, view,
    fetchContacts, fetchGroups,
    setSelectedId,
    editorOpen, setEditorOpen,
    importOpen, setImportOpen,
  } = useContactsStore()

  const [dismissedBanner, setDismissedBanner] = useState(false)
  const [rightPanelOpen, setRightPanelOpen]   = useState(true)

  useEffect(() => {
    fetchContacts()
    fetchGroups()
  }, [])

  const selected = contacts.find(c => c.id === selectedId) ?? null

  const viewLabel: Record<string, string> = {
    all:        t('title_all'),
    starred:    t('title_starred'),
    directory:  t('title_directory'),
    trashed:    t('title_trashed'),
    group:      t('title_group'),
    duplicates: t('title_duplicates'),
  }
  const title = viewLabel[view] ?? t('title_all')

  if (editorOpen) {
    return (
      <div className="flex-1 overflow-hidden">
        <ContactEditor onDone={() => { setEditorOpen(false); fetchContacts() }} />
      </div>
    )
  }

  if (view === 'directory') {
    return (
      <div className="flex-1 overflow-hidden">
        <DirectoryView />
      </div>
    )
  }

  const isEmpty = !contacts.length

  // Group contacts alphabetically
  const grouped = new Map<string, Contact[]>()
  for (const c of contacts) {
    const first  = c.display_name?.[0]?.toUpperCase() ?? '#'
    const letter = /[A-Z]/.test(first) ? first : '#'
    if (!grouped.has(letter)) grouped.set(letter, [])
    grouped.get(letter)!.push(c)
  }
  const sortedKeys = [...grouped.keys()].sort((a, b) =>
    a === '#' ? 1 : b === '#' ? -1 : a.localeCompare(b)
  )

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* ── Left: table area ─────────────────────────────────────────────── */}
      <div className="flex-1 flex flex-col overflow-hidden">

        {/* Duplicate banner */}
        {view === 'all' && !dismissedBanner && contacts.length > 5 && (
          <div className="mx-4 mt-3 flex items-center gap-3 px-4 py-3 rounded-xl text-sm"
               style={{ background: '#e8f0fe' }}>
            <span className="flex-1 text-text-primary">
              {t('contacts_dup_banner')}
            </span>
            <button
              onClick={() => setDismissedBanner(true)}
              className="text-primary font-medium hover:underline"
            >
              {t('contacts_dismiss')}
            </button>
            <button
              onClick={() => useContactsStore.getState().setView('duplicates')}
              className="text-primary font-medium hover:underline"
            >
              {t('contacts_show')}
            </button>
          </div>
        )}

        {/* Header */}
        <div className="px-6 pt-4 pb-2 flex items-center gap-3 flex-shrink-0">
          <h1 className="text-2xl font-normal text-text-primary flex-1">
            {title}
            {total > 0 && (
              <span className="text-text-secondary ml-2 text-xl">({total})</span>
            )}
          </h1>
          {!isEmpty && (
            <div className="flex items-center gap-1">
              <button className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary transition-colors" title={t('print')}>
                <Printer size={16} />
              </button>
              <button
                onClick={() => setImportOpen(true)}
                className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary transition-colors"
                title={t('export')}
              >
                <Upload size={16} />
              </button>
              <button className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary transition-colors" title={t('more_options')}>
                <MoreHorizontal size={16} />
              </button>
            </div>
          )}
        </div>

        {isEmpty ? (
          /* ── Empty state ─────────────────────────────────────────────── */
          <div className="flex-1 flex flex-col items-center justify-center gap-4">
            <EmptyIllustration />
            <p className="text-base text-text-primary">{t('empty')}</p>
            <div className="flex items-center gap-8">
              <button
                onClick={() => setEditorOpen(true)}
                className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"
              >
                <UserPlus size={16} />
                {t('create_contact')}
              </button>
              <button
                onClick={() => setImportOpen(true)}
                className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"
              >
                <Download size={16} />
                {t('import_contacts')}
              </button>
            </div>
          </div>
        ) : (
          /* ── Table ───────────────────────────────────────────────────── */
          <div className="flex-1 overflow-y-auto">
            {/* Column headers */}
            <div
              className="grid px-4 py-2 border-b border-border sticky top-0 bg-white z-10"
              style={{ gridTemplateColumns: '48px 1fr 1fr 1fr' }}
            >
              <div />
              <span className="text-sm font-medium text-text-secondary">{t('col_name')}</span>
              <span className="text-sm font-medium text-text-secondary">{t('col_email')}</span>
              <span className="text-sm font-medium text-text-secondary">{t('col_phone')}</span>
            </div>

            {sortedKeys.map(letter => (
              <div key={letter}>
                <div
                  className="px-4 py-1.5 text-xs font-semibold text-text-secondary"
                  style={{ gridTemplateColumns: '48px 1fr 1fr 1fr' }}
                >
                  {letter}
                </div>
                {grouped.get(letter)!.map(c => (
                  <ContactRow
                    key={c.id}
                    contact={c}
                    isSelected={c.id === selectedId}
                    onClick={() => setSelectedId(c.id === selectedId ? null : c.id)}
                  />
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ── Right: detail panel ──────────────────────────────────────────── */}
      {selected && (
        <div
          className="flex-shrink-0 border-l border-border overflow-y-auto transition-[width] duration-200"
          style={{ width: rightPanelOpen ? '360px' : '0px' }}
        >
          {rightPanelOpen && (
            <div className="relative">
              <button
                onClick={() => { setSelectedId(null); setRightPanelOpen(true) }}
                className="absolute top-3 right-3 w-8 h-8 rounded-full flex items-center justify-center
                           hover:bg-surface-2 text-text-secondary transition-colors z-10"
              >
                <X size={16} />
              </button>
              <ContactDetail contact={selected} />
            </div>
          )}
        </div>
      )}

      {importOpen && <ImportModal onClose={() => setImportOpen(false)} />}
    </div>
  )
}
