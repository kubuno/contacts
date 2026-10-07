import { useTranslation } from 'react-i18next'
import { Star } from 'lucide-react'
import { Contact } from './api'
import ContactAvatar from './ContactAvatar'

interface Props {
  contact: Contact
  isSelected: boolean
  onClick: () => void
}

export default function ContactListItem({ contact, isSelected, onClick }: Props) {
  const { t } = useTranslation('contacts')
  const subtitle = contact.organization || contact.emails[0]?.value || contact.phones[0]?.value || ''

  return (
    <button
      className={`w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-2 transition-colors ${isSelected ? 'bg-primary-light' : ''}`}
      onClick={onClick}
    >
      <ContactAvatar contact={contact} size="sm" />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1">
          <span className={`text-sm font-medium truncate ${isSelected ? 'text-primary' : 'text-text-primary'}`}>
            {contact.display_name || t('no_name')}
          </span>
          {contact.is_starred && <Star size={11} className="text-yellow-500 fill-yellow-500 flex-shrink-0" />}
        </div>
        {subtitle && (
          <p className="text-xs text-text-secondary truncate">{subtitle}</p>
        )}
      </div>
    </button>
  )
}
