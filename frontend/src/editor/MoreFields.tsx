import { useTranslation } from 'react-i18next'
import { CalendarPlus, Link2, UserRound, ListPlus } from 'lucide-react'
import { OutlinedField, DateField, type DateValue } from '@ui'
import FieldRow from './FieldRow'
import AddAction from './AddAction'
import { EMPTY_DATE } from './dateValue'
import type { DateRow, UrlRow, RelRow } from './useContactForm'
import type { CustomField } from '../api'

/**
 * The fields folded behind "Plus": extra dates, websites, related people and
 * free-form custom fields. They are real parts of the record, just rarely filled
 * — showing them all at rest would bury the fields that matter.
 */
export default function MoreFields({
  dates, setDates, urls, setUrls, relations, setRelations, custom, setCustom, accent, wide,
}: {
  dates:        DateRow[]
  setDates:     (v: DateRow[]) => void
  urls:         UrlRow[]
  setUrls:      (v: UrlRow[]) => void
  relations:    RelRow[]
  setRelations: (v: RelRow[]) => void
  custom:       CustomField[]
  setCustom:    (v: CustomField[]) => void
  accent:       string
  /** Spread the four blocks over two columns on a wide screen. */
  wide?:        boolean
}) {
  const { t } = useTranslation('contacts')
  const datePresets = [t('contacts_type_anniversary'), t('contacts_type_other')]

  return (
    <div className={wide ? 'grid grid-cols-2 gap-x-12 gap-y-5 items-start' : 'space-y-5'}>
      {/* Extra dates (anniversary…): the birthday has its own row above. */}
      <div className="space-y-2">
        {dates.map((d, i) => (
          <FieldRow key={i} icon={i === 0 ? <CalendarPlus size={20} /> : undefined}
            onRemove={() => setDates(dates.filter((_, j) => j !== i))}>
            <DateField value={d.date} primaryColor={accent} icon={null}
              withLabel={!!(d.date.day.trim() || d.date.month.trim())} labelPresets={datePresets}
              onChange={(v: DateValue) => setDates(dates.map((x, j) => j === i ? { date: v } : x))} />
          </FieldRow>
        ))}
        <AddAction label={t('ed_add_date')} icon={<CalendarPlus size={16} />} filled={dates.length === 0}
          onClick={() => setDates([...dates, { date: { ...EMPTY_DATE, label: '' } }])} />
      </div>

      <div className="space-y-2">
        {urls.map((u, i) => (
          <FieldRow key={i} icon={i === 0 ? <Link2 size={20} /> : undefined}
            onRemove={() => setUrls(urls.filter((_, j) => j !== i))}>
            <OutlinedField label={t('contacts_field_website')} value={u.value} type="url" inputMode="url"
              primaryColor={accent} onChange={v => setUrls(urls.map((x, j) => j === i ? { ...x, value: v } : x))} />
          </FieldRow>
        ))}
        <AddAction label={t('ed_add_url')} icon={<Link2 size={16} />} filled={urls.length === 0}
          onClick={() => setUrls([...urls, { value: '', type: 'work' }])} />
      </div>

      <div className="space-y-2">
        {relations.map((r, i) => (
          <FieldRow key={i} icon={i === 0 ? <UserRound size={20} /> : undefined}
            onRemove={() => setRelations(relations.filter((_, j) => j !== i))}>
            <OutlinedField label={t('ed_relation_name')} value={r.value} primaryColor={accent}
              onChange={v => setRelations(relations.map((x, j) => j === i ? { ...x, value: v } : x))} />
          </FieldRow>
        ))}
        <AddAction label={t('ed_add_relation')} icon={<UserRound size={16} />} filled={relations.length === 0}
          onClick={() => setRelations([...relations, { value: '', type: 'other' }])} />
      </div>

      <div className="space-y-2">
        {custom.map((c, i) => (
          <FieldRow key={i} icon={i === 0 ? <ListPlus size={20} /> : undefined}
            onRemove={() => setCustom(custom.filter((_, j) => j !== i))}>
            <div className="space-y-2">
              <OutlinedField label={t('ed_custom_label')} value={c.label} primaryColor={accent}
                onChange={v => setCustom(custom.map((x, j) => j === i ? { ...x, label: v } : x))} />
              <OutlinedField label={t('ed_custom_value')} value={c.value} primaryColor={accent}
                onChange={v => setCustom(custom.map((x, j) => j === i ? { ...x, value: v } : x))} />
            </div>
          </FieldRow>
        ))}
        <AddAction label={t('ed_add_custom')} icon={<ListPlus size={16} />} filled={custom.length === 0}
          onClick={() => setCustom([...custom, { label: '', value: '' }])} />
      </div>
    </div>
  )
}
