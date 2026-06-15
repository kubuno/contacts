import { useTranslation } from 'react-i18next'
import { useContactsStore } from './store'
import ContactListItem from './ContactListItem'

export default function ContactList() {
  const { t } = useTranslation('contacts')
  const { contacts, selectedId, setSelectedId, isLoading } = useContactsStore()

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-text-secondary">
        {t('common_loading')}
      </div>
    )
  }

  if (contacts.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2 text-text-secondary p-8">
        <p className="text-sm">{t('contacts_none')}</p>
      </div>
    )
  }

  // Group alphabetically
  const grouped = new Map<string, typeof contacts>()
  for (const c of contacts) {
    const letter = c.display_name?.[0]?.toUpperCase() || '#'
    const key = /[A-Z]/.test(letter) ? letter : '#'
    if (!grouped.has(key)) grouped.set(key, [])
    grouped.get(key)!.push(c)
  }
  const sortedKeys = [...grouped.keys()].sort((a, b) => a === '#' ? 1 : b === '#' ? -1 : a.localeCompare(b))

  return (
    <div className="flex-1 overflow-y-auto">
      {sortedKeys.map(letter => (
        <div key={letter}>
          <div className="sticky top-0 bg-surface-1 px-4 py-1 text-xs font-semibold text-text-secondary border-b border-border z-10">
            {letter}
          </div>
          {grouped.get(letter)!.map(c => (
            <ContactListItem
              key={c.id}
              contact={c}
              isSelected={c.id === selectedId}
              onClick={() => setSelectedId(c.id)}
            />
          ))}
        </div>
      ))}
    </div>
  )
}
