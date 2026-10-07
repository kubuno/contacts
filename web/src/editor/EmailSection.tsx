import { useTranslation } from 'react-i18next'
import { Mail } from 'lucide-react'
import { OutlinedField } from '@ui'
import FieldRow from './FieldRow'
import AddAction from './AddAction'
import type { EmailRow } from './useContactForm'

/** E-mail addresses: one Material field per address, the icon on the first row
 *  only, a cross on hover to drop one. */
export default function EmailSection({ rows, onChange, accent }: {
  rows:     EmailRow[]
  onChange: (rows: EmailRow[]) => void
  accent:   string
}) {
  const { t } = useTranslation('contacts')
  const set = (i: number, v: Partial<EmailRow>) => onChange(rows.map((r, j) => j === i ? { ...r, ...v } : r))

  return (
    <div className="space-y-2">
      {rows.map((r, i) => (
        <FieldRow key={i} icon={i === 0 ? <Mail size={20} /> : undefined}
          onRemove={() => onChange(rows.filter((_, j) => j !== i))}>
          <OutlinedField label={t('contacts_field_email')} value={r.value} type="email" inputMode="email"
            primaryColor={accent} onChange={v => set(i, { value: v })} />
        </FieldRow>
      ))}
      <AddAction label={t('ed_add_email')} icon={<Mail size={16} />} filled={rows.length === 0}
        onClick={() => onChange([...rows, { value: '', type: 'work' }])} />
    </div>
  )
}
