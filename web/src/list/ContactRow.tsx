import { useTranslation } from 'react-i18next'
import ContactAvatar from '../ContactAvatar'
import { RowCheckbox, LabelChips } from '../widgets'
import { useContactMenu } from '../contactMenu'
import type { Contact } from '../api'

/**
 * A contact in the list view. Desktop lays name / email / phone across columns;
 * on a phone the columns collapse to a single stack — a taller row (better touch
 * target) with the email as a secondary line under the name, since the email and
 * phone columns are hidden.
 */
export default function ContactRow({ contact, isSelected, isChecked, isMobile, onClick }: {
  contact:    Contact
  isSelected: boolean
  isChecked:  boolean
  isMobile:   boolean
  onClick:    () => void
}) {
  const { t } = useTranslation('contacts')
  const { open } = useContactMenu()
  const email = contact.emails[0]?.value ?? ''
  const phone = contact.phones[0]?.value ?? ''

  if (isMobile) {
    return (
      <div
        onClick={onClick}
        onContextMenu={e => open(e, contact)}
        className={`w-full flex items-center gap-3 px-4 min-h-[56px] py-2 active:bg-surface-2 transition-colors cursor-pointer
                    ${isSelected ? 'bg-primary-light' : isChecked ? 'bg-primary-light/40' : ''}`}
      >
        <ContactAvatar contact={contact} size="md" />
        <div className="flex-1 min-w-0">
          <span className={`block text-[15px] truncate flex items-center gap-2 ${isSelected ? 'text-primary font-medium' : 'text-text-primary'}`}>
            {contact.display_name || t('no_name')}
            <LabelChips labelIds={contact.label_ids} />
          </span>
          {email && <span className="block text-[13px] text-text-secondary truncate">{email}</span>}
        </div>
      </div>
    )
  }

  return (
    <div
      onClick={onClick}
      onContextMenu={e => open(e, contact)}
      className={`w-full grid items-center px-4 py-2 hover:bg-surface-1 transition-colors group text-left cursor-pointer
                  grid-cols-[36px_40px_1.4fr_1.4fr_1fr]
                  ${isSelected ? 'bg-primary-light' : isChecked ? 'bg-primary-light/40' : ''}`}
    >
      <div className={isChecked ? '' : 'opacity-0 group-hover:opacity-100'}><RowCheckbox contact={contact} checked={isChecked} /></div>
      <ContactAvatar contact={contact} size="sm" />
      <span className={`text-sm truncate pr-4 flex items-center gap-2 ${isSelected ? 'text-primary font-medium' : 'text-text-primary'}`}>
        {contact.display_name || t('no_name')}
        <LabelChips labelIds={contact.label_ids} />
      </span>
      <span className="text-sm text-text-secondary truncate pr-4">{email}</span>
      <span className="text-sm text-text-secondary truncate">{phone}</span>
    </div>
  )
}
