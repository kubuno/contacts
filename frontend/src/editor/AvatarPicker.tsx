import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { pickImageFile } from '@kubuno/sdk'
import { ColorField } from '@ui'
import ContactAvatar from '../ContactAvatar'
import type { Contact } from '../api'

/**
 * The photo at the head of the editor, at the size it has in Google Contacts —
 * large enough to be the subject of the form rather than a thumbnail. A round
 * "+" badge sits on its lower-right corner and carries the affordance on its
 * own, instead of a caption that only appears on hover (and never appears at all
 * on a touch screen).
 *
 * The colour control is ours, not Google's: a contact with no photo is drawn from
 * its initials, and the colour is what tells two of them apart. It uses the
 * platform's own colour picker — full spectrum, harmonies and remembered custom
 * colours — instead of a handful of frozen swatches. Its `t` is left alone on
 * purpose: the picker ships its own labels, and handing it this module's
 * translator would make it print raw keys it has never heard of.
 */
export default function AvatarPicker({ preview, color, onPickFile, onColor }: {
  preview:    Contact
  color:      string
  onPickFile: (file: File | null) => void
  onColor:    (c: string) => void
}) {
  const { t } = useTranslation('contacts')
  const choose = () => { void pickImageFile({ title: t('contacts_photo') }).then(onPickFile) }

  return (
    <div className="flex items-end gap-6">
      <div className="relative flex-shrink-0">
        <button type="button" onClick={choose} className="cursor-pointer block rounded-full" aria-label={t('contacts_photo')}>
          <ContactAvatar contact={preview} size="2xl" />
        </button>
        <button
          type="button"
          onClick={choose}
          title={t('contacts_photo')}
          aria-label={t('contacts_photo')}
          className="absolute bottom-2 right-2 w-11 h-11 rounded-full bg-primary text-white
                     flex items-center justify-center border-4 border-white
                     hover:bg-primary-hover transition-colors"
        >
          <Plus size={22} />
        </button>
      </div>

      <div className="pb-3">
        <p className="text-xs text-text-secondary mb-2">{t('contacts_color')}</p>
        <ColorField
          color={color}
          onChange={onColor}
          width={40}
          height={40}
          className="rounded-full"
        />
      </div>
    </div>
  )
}
