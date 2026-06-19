import { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { MenuDropdown, type MenuItem, type MenuDropdownPos } from '@ui'
import {
  Trash2, Star, Tag, Users, Download, Archive, X, CheckSquare, RotateCcw,
} from 'lucide-react'
import { Contact, contactsApi } from './api'
import { useContactsStore } from './store'

/** Clipboard copy helper with a transient "copied" flag. */
export function useCopy(): [boolean, (text: string) => void] {
  const [copied, setCopied] = useState(false)
  const copy = useCallback((text: string) => {
    navigator.clipboard?.writeText(text).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    })
  }, [])
  return [copied, copy]
}

/** Small colored label chip. */
export function LabelChip({ name, color, onRemove }: { name: string; color: string; onRemove?: () => void }) {
  return (
    <span
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
      style={{ backgroundColor: color + '22', color }}
    >
      <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: color }} />
      {name}
      {onRemove && (
        <button onClick={onRemove} className="hover:opacity-70 ml-0.5"><X size={10} /></button>
      )}
    </span>
  )
}

/** Renders the label chips attached to a contact (from the store's label list). */
export function LabelChips({ labelIds }: { labelIds?: string[] }) {
  const labels = useContactsStore(s => s.labels)
  if (!labelIds?.length) return null
  const mine = labels.filter(l => labelIds.includes(l.id))
  if (!mine.length) return null
  return (
    <span className="inline-flex flex-wrap gap-1">
      {mine.map(l => <LabelChip key={l.id} name={l.name} color={l.color} />)}
    </span>
  )
}

/** Floating bulk-action bar shown when one or more contacts are selected. */
export function SelectionBar() {
  const { t } = useTranslation('contacts')
  const { selectedIds, clearSelection, selectAll, contacts, fetchContacts, fetchLabels, view } = useContactsStore()
  const labels = useContactsStore(s => s.labels)
  const groups = useContactsStore(s => s.groups)
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)

  if (selectedIds.size === 0) return null
  const ids = [...selectedIds]

  async function run(action: string) {
    await contactsApi.bulk(ids, action)
    clearSelection()
    fetchContacts()
  }

  async function assignLabel(labelId: string) {
    await contactsApi.addLabelMembers(labelId, ids)
    clearSelection(); fetchContacts(); fetchLabels()
  }
  async function assignGroup(groupId: string) {
    await contactsApi.addGroupMembers(groupId, ids)
    clearSelection(); fetchContacts()
  }

  function openLabelMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setMenu({
      pos: { top: r.bottom + 4, left: r.left },
      items: labels.length
        ? labels.map(l => ({ type: 'action' as const, label: l.name,
            icon: <span className="w-3 h-3 rounded-full inline-block" style={{ backgroundColor: l.color }} />,
            onClick: () => assignLabel(l.id) }))
        : [{ type: 'label' as const, text: t('rem_empty') }],
    })
  }
  function openGroupMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setMenu({
      pos: { top: r.bottom + 4, left: r.left },
      items: groups.length
        ? groups.map(g => ({ type: 'action' as const, label: g.name,
            icon: <span className="w-3 h-3 rounded-sm inline-block" style={{ backgroundColor: g.color }} />,
            onClick: () => assignGroup(g.id) }))
        : [{ type: 'label' as const, text: t('rem_empty') }],
    })
  }

  const Btn = ({ icon, label, onClick }: { icon: React.ReactNode; label: string; onClick: (e: React.MouseEvent) => void }) => (
    <button onClick={onClick} title={label}
      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg hover:bg-surface-2 text-sm text-text-primary transition-colors">
      {icon}<span className="hidden md:inline">{label}</span>
    </button>
  )

  return (
    <div className="mx-4 mt-3 flex items-center gap-1 px-3 py-2 rounded-xl bg-primary-light shadow-sm">
      <button onClick={clearSelection} className="p-1.5 rounded-full hover:bg-white/50" title={t('sel_clear')}>
        <X size={16} />
      </button>
      <span className="text-sm font-medium text-primary mr-2">{t('sel_count', { count: selectedIds.size })}</span>
      <button onClick={selectAll} className="p-1.5 rounded-full hover:bg-white/50" title={t('sel_all')}>
        <CheckSquare size={16} />
      </button>
      <div className="w-px h-5 bg-primary/20 mx-1" />
      {view === 'trashed' ? (
        <>
          <Btn icon={<RotateCcw size={16} />} label={t('sel_restore')} onClick={() => run('restore')} />
          <Btn icon={<Trash2 size={16} />} label={t('sel_delete')} onClick={() => run('delete')} />
        </>
      ) : (
        <>
          <Btn icon={<Star size={16} />} label={t('sel_star')} onClick={() => run('star')} />
          <Btn icon={<Tag size={16} />} label={t('sel_label')} onClick={openLabelMenu} />
          <Btn icon={<Users size={16} />} label={t('sel_group')} onClick={openGroupMenu} />
          <Btn icon={<Archive size={16} />} label={t('sel_archive')} onClick={() => run(view === 'archived' ? 'unarchive' : 'archive')} />
          <Btn icon={<Trash2 size={16} />} label={t('sel_delete')} onClick={() => run('trash')} />
        </>
      )}
      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
    </div>
  )
}

/** A selectable avatar/checkbox combo used in list rows. */
export function RowCheckbox({ contact, checked }: { contact: Contact; checked: boolean }) {
  const toggleSelect = useContactsStore(s => s.toggleSelect)
  return (
    <button
      onClick={(e) => { e.stopPropagation(); toggleSelect(contact.id, e.shiftKey) }}
      className={`w-5 h-5 rounded border flex items-center justify-center transition-colors
                  ${checked ? 'bg-primary border-primary' : 'border-border hover:border-primary'}`}
    >
      {checked && <CheckSquare size={12} className="text-white" />}
    </button>
  )
}
