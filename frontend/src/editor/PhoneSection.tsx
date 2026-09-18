import { useTranslation } from 'react-i18next'
import { Phone } from 'lucide-react'
import { PhoneField } from '@ui'
import FieldRow from './FieldRow'
import AddAction from './AddAction'
import { DEFAULT_COUNTRY } from './phoneValue'
import type { PhoneRow } from './useContactForm'

/** Phone numbers, each with its country dial-code selector and an editable
 *  label (Mobile, Domicile…) — the shared `PhoneField` primitive. */
export default function PhoneSection({ rows, onChange, accent }: {
  rows:     PhoneRow[]
  onChange: (rows: PhoneRow[]) => void
  accent:   string
}) {
  const { t } = useTranslation('contacts')
  const presets = [
    t('contacts_type_mobile'), t('contacts_type_home'),
    t('contacts_type_work'), t('contacts_type_fax'), t('contacts_type_other'),
  ]

  return (
    <div className="space-y-2">
      {rows.map((r, i) => (
        <FieldRow key={i} icon={i === 0 ? <Phone size={20} /> : undefined}
          onRemove={() => onChange(rows.filter((_, j) => j !== i))}>
          {/* icon={null}: the row's gutter already carries the single handset
              icon — the primitive would otherwise draw a second one. The label
              combobox only appears once a number is typed, so an untouched form
              stays as sparse as Google's. */}
          <PhoneField value={r.phone} primaryColor={accent} icon={null}
            withLabel={!!r.phone.number.trim()} labelPresets={presets}
            onChange={v => onChange(rows.map((x, j) => j === i ? { phone: v } : x))} />
        </FieldRow>
      ))}
      <AddAction label={t('ed_add_phone')} icon={<Phone size={16} />} filled={rows.length === 0}
        onClick={() => onChange([...rows, { phone: { country: DEFAULT_COUNTRY, number: '', label: '' } }])} />
    </div>
  )
}
