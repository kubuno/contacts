import { useTranslation } from 'react-i18next'
import ContactAvatar from '../ContactAvatar'
import { RowCheckbox, LabelChips } from '../widgets'
import { useContactMenu } from '../contactMenu'
import type { Contact } from '../api'

/** A contact as a card in the grid view (avatar over name, org and email). */
export default function ContactCard({ contact, isSelected, isChecked, onClick }: {
  contact:    Contact
  isSelected: boolean
  isChecked:  boolean
  onClick:    () => void
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
