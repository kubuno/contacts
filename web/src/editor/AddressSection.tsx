import { useTranslation } from 'react-i18next'
import { MapPin } from 'lucide-react'
import { OutlinedField } from '@ui'
import FieldRow from './FieldRow'
import AddAction from './AddAction'
import type { AddressField } from '../api'

/** Postal addresses. An address is several sub-fields, so each one is a stacked
 *  block under the pin icon — the same shape the Name and Organisation groups
 *  use, minus the chevron (every part of an address is worth showing). */
export default function AddressSection({ rows, onChange, accent }: {
  rows:     AddressField[]
  onChange: (rows: AddressField[]) => void
  accent:   string
}) {
  const { t } = useTranslation('contacts')
  const set = (i: number, patch: Partial<AddressField>) =>
    onChange(rows.map((a, j) => j === i ? { ...a, ...patch } : a))

  const PARTS: Array<[keyof AddressField, string]> = [
    ['street',   t('ed_street')],
    ['city',     t('ed_city')],
    ['postcode', t('ed_postcode')],
    ['region',   t('ed_region')],
    ['country',  t('ed_country')],
  ]

  return (
    <div className="space-y-2">
      {rows.map((a, i) => (
        <FieldRow key={i} icon={<MapPin size={20} />}
          onRemove={() => onChange(rows.filter((_, j) => j !== i))}>
          <div className="space-y-2">
            {PARTS.map(([key, label]) => (
              <OutlinedField key={String(key)} label={label} primaryColor={accent}
                value={(a[key] as string) ?? ''} onChange={v => set(i, { [key]: v } as Partial<AddressField>)} />
            ))}
          </div>
        </FieldRow>
      ))}
      <AddAction label={t('ed_add_address')} icon={<MapPin size={16} />} filled={rows.length === 0}
        onClick={() => onChange([...rows, { type: 'home' }])} />
    </div>
  )
}
