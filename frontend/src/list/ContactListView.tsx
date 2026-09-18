import { useTranslation } from 'react-i18next'
import ContactRow from './ContactRow'
import ContactCard from './ContactCard'
import { RowCheckbox } from '../widgets'
import { useContactMenu } from '../contactMenu'
import type { ViewMode } from '../store'
import type { Contact as ApiContact } from '../api'

/**
 * The body of the list area, in whichever density is active. `table` is a
 * desktop mode; on a phone it degrades to the (touch-sized) list, so a value
 * left over from the desktop never renders an unusable table on mobile.
 */
export default function ContactListView({ contacts, viewMode, selectedId, selectedIds, isMobile, onSelect }: {
  contacts:    ApiContact[]
  viewMode:    ViewMode
  selectedId:  string | null
  selectedIds: Set<string>
  isMobile:    boolean
  onSelect:    (id: string) => void
}) {
  const { t } = useTranslation('contacts')
  const { open: openContactMenu } = useContactMenu()
  const mode: ViewMode = isMobile && viewMode === 'table' ? 'list' : viewMode

  if (mode === 'grid') {
    return (
      <div className="flex-1 overflow-y-auto px-4 pb-6">
        <div className="grid gap-3" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))' }}>
          {contacts.map(c => (
            <ContactCard key={c.id} contact={c} isSelected={c.id === selectedId} isChecked={selectedIds.has(c.id)}
              onClick={() => onSelect(c.id)} />
          ))}
        </div>
      </div>
    )
  }

  if (mode === 'table') {
    return (
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-white border-b border-border">
            <tr className="text-left text-text-secondary">
              <th className="px-4 py-2 font-medium">{t('col_name')}</th>
              <th className="px-4 py-2 font-medium">{t('col_email')}</th>
              <th className="px-4 py-2 font-medium">{t('col_phone')}</th>
              <th className="px-4 py-2 font-medium">{t('sort_org')}</th>
            </tr>
          </thead>
          <tbody>
            {contacts.map(c => (
              <tr key={c.id} onClick={() => onSelect(c.id)} onContextMenu={e => openContactMenu(e, c)}
                className={`cursor-pointer hover:bg-surface-1 border-b border-border/50 ${c.id === selectedId ? 'bg-primary-light' : selectedIds.has(c.id) ? 'bg-primary-light/40' : ''}`}>
                <td className="px-4 py-2 flex items-center gap-2"><RowCheckbox contact={c} checked={selectedIds.has(c.id)} />{c.display_name || t('no_name')}</td>
                <td className="px-4 py-2 text-text-secondary">{c.emails[0]?.value ?? ''}</td>
                <td className="px-4 py-2 text-text-secondary">{c.phones[0]?.value ?? ''}</td>
                <td className="px-4 py-2 text-text-secondary">{c.organization ?? ''}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }

  // List: alphabetical sections by first letter of the display name.
  const grouped = new Map<string, ApiContact[]>()
  for (const c of contacts) {
    const first = c.display_name?.[0]?.toUpperCase() ?? '#'
    const letter = /[A-Z]/.test(first) ? first : '#'
    if (!grouped.has(letter)) grouped.set(letter, [])
    grouped.get(letter)!.push(c)
  }
  const sortedKeys = [...grouped.keys()].sort((a, b) => a === '#' ? 1 : b === '#' ? -1 : a.localeCompare(b))

  return (
    <div className="flex-1 overflow-y-auto">
      {!isMobile && (
        <div className="grid px-4 py-2 border-b border-border sticky top-0 bg-white z-10 grid-cols-[36px_40px_1.4fr_1.4fr_1fr]">
          <div /><div />
          <span className="text-sm font-medium text-text-secondary">{t('col_name')}</span>
          <span className="text-sm font-medium text-text-secondary">{t('col_email')}</span>
          <span className="text-sm font-medium text-text-secondary">{t('col_phone')}</span>
        </div>
      )}
      {sortedKeys.map(letter => (
        <div key={letter}>
          <div className="px-4 py-1.5 text-xs font-semibold text-text-secondary sticky top-0 bg-white/95 backdrop-blur-sm z-[5]">{letter}</div>
          {grouped.get(letter)!.map(c => (
            <ContactRow key={c.id} contact={c} isSelected={c.id === selectedId} isChecked={selectedIds.has(c.id)}
              isMobile={isMobile} onClick={() => onSelect(c.id)} />
          ))}
        </div>
      ))}
    </div>
  )
}
