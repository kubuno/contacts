import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus, X, Tag } from 'lucide-react'
import { MenuDropdown, Input, type MenuItem, type MenuDropdownPos } from '@ui'
import { useContactsStore } from '../store'
import { contactsApi } from '../api'

const LABEL_COLORS = ['#1a73e8', '#e8710a', '#1e8e3e', '#d93025', '#9334e6', '#12b5cb', '#f9ab00', '#e52592']

/**
 * The "+ Libellé" chip. Labels are membership rather than a column, so the form
 * only collects the chosen ids and applies the difference once the contact
 * exists — a brand-new contact has no id to attach a label to yet.
 *
 * The chip is ALWAYS offered, even on an instance with no label at all: its menu
 * ends with "create a label", which turns into an inline name field (Google's
 * flow is exactly that — type a name, save). Hiding the chip when the list is
 * empty would leave a new user with no way in from here.
 */
export default function LabelPicker({ selected, onChange }: {
  selected: string[]
  onChange: (ids: string[]) => void
}) {
  const { t } = useTranslation('contacts')
  const { labels, fetchLabels } = useContactsStore()
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)
  const [creating, setCreating] = useState(false)
  const [name, setName] = useState('')

  const chosen = labels.filter(l => selected.includes(l.id))

  async function createLabel() {
    const n = name.trim()
    if (!n) return
    const color = LABEL_COLORS[labels.length % LABEL_COLORS.length]
    const res = await contactsApi.createLabel(n, color)
    const created = (res.data as { label?: { id: string } }).label
    await fetchLabels()
    // Attach it straight away: creating a label from this form only ever means
    // "put this contact in it".
    if (created?.id) onChange([...selected, created.id])
    setName(''); setCreating(false)
  }

  function openMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const items: MenuItem[] = labels.map(l => ({
      type: 'action',
      label: l.name,
      checked: selected.includes(l.id),
      icon: <span className="w-3 h-3 rounded-full inline-block" style={{ backgroundColor: l.color }} />,
      onClick: () => onChange(selected.includes(l.id) ? selected.filter(id => id !== l.id) : [...selected, l.id]),
    }))
    if (labels.length) items.push({ type: 'separator' })
    items.push({ type: 'action', label: t('ed_create_label'), icon: <Plus size={15} />, onClick: () => setCreating(true) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items })
  }

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {chosen.map(l => (
        <span key={l.id} className="inline-flex items-center gap-1.5 pl-2 pr-1 py-1 rounded-full text-sm border border-border">
          <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: l.color }} />
          {l.name}
          <button type="button" onClick={() => onChange(selected.filter(id => id !== l.id))}
            aria-label={t('common_delete')} className="w-5 h-5 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary">
            <X size={13} />
          </button>
        </span>
      ))}

      {creating ? (
        <span className="inline-flex items-center gap-2">
          <Input autoFocus type="text" value={name} onChange={e => setName(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); void createLabel() }
                              if (e.key === 'Escape') { setCreating(false); setName('') } }}
            placeholder={t('label_name')} className="w-44" />
          <button type="button" onClick={() => void createLabel()} className="text-sm text-primary hover:underline">
            {t('common_save')}
          </button>
        </span>
      ) : (
        <button type="button" onClick={openMenu}
          className="inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-sm border border-border text-primary hover:bg-surface-1">
          {chosen.length ? <Tag size={14} /> : <Plus size={14} />}{t('ed_label')}
        </button>
      )}

      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
    </div>
  )
}
