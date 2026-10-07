import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ArrowLeft, X, User, Building2, Cake, FileText, ChevronDown, ChevronUp, Star } from 'lucide-react'
import { Button, FieldGroup, DateField, OutlinedField, type GroupField, type DateValue } from '@ui'
import { Contact } from '../api'
import { useIsMobile, useIsWide } from '../hooks/useMediaQuery'
import AvatarPicker from './AvatarPicker'
import LabelPicker from './LabelPicker'
import EmailSection from './EmailSection'
import PhoneSection from './PhoneSection'
import AddressSection from './AddressSection'
import MoreFields from './MoreFields'
import { useContactForm } from './useContactForm'

interface Props {
  contact?: Contact
  onDone:   () => void
}

// The module accent, resolved at runtime from the theme, so the Material fields
// float and underline in the contacts colour and follow a runtime theme.
const ACCENT = 'var(--color-primary)'

export default function ContactEditor({ contact, onDone }: Props) {
  const { t } = useTranslation('contacts')
  const isMobile = useIsMobile()
  // Two columns once the area is wide enough: a single 576px column on a large
  // screen leaves most of the page empty for no benefit.
  const isWide = useIsWide()
  const f = useContactForm(contact, onDone)
  const [showMore, setShowMore] = useState(false)

  // Name: two fields at rest, the rest folded behind the chevron — Google's
  // split, mapped onto the fields this module actually stores. (Phonetic names
  // and "file as" have no column here, so they are absent rather than faked.)
  const NAME_FIELDS: GroupField[] = [
    { key: 'name_prefix', label: t('contacts_field_prefix'),      advanced: true },
    { key: 'given_name',  label: t('contacts_field_given_name') },
    { key: 'middle_name', label: t('contacts_field_middle_name'), advanced: true },
    { key: 'family_name', label: t('contacts_field_family_name') },
    { key: 'name_suffix', label: t('contacts_field_suffix'),      advanced: true },
    { key: 'nickname',    label: t('contacts_field_nickname'),    advanced: true },
    { key: 'pronouns',    label: t('ed_pronouns'),                advanced: true },
  ]
  const ORG_FIELDS: GroupField[] = [
    { key: 'organization', label: t('contacts_field_company') },
    { key: 'job_title',    label: t('contacts_field_job_title') },
    { key: 'department',   label: t('contacts_field_department'), advanced: true },
  ]
  const nameValues = {
    name_prefix: f.form.name_prefix, given_name: f.form.given_name, middle_name: f.form.middle_name,
    family_name: f.form.family_name, name_suffix: f.form.name_suffix, nickname: f.form.nickname,
    pronouns: f.form.pronouns,
  }
  const orgValues = { organization: f.form.organization, job_title: f.form.job_title, department: f.form.department }

  const previewContact = (contact
    ? { ...contact, display_name: f.displayName || t('contacts_new_contact'), avatar_color: f.form.avatar_color, avatar_path: f.avatarPreview ?? contact.avatar_path }
    : { id: '', display_name: f.displayName || t('contacts_new_contact'), avatar_color: f.form.avatar_color, avatar_path: f.avatarPreview, emails: [], phones: [] }) as unknown as Contact

  return (
    // The form lives INSIDE the module's content area, like Google Contacts: the
    // shell keeps its sidebar and header around it. It is not an overlay — the
    // editor is a view of this module, not a window on top of the application.
    <div className="h-full overflow-y-auto bg-white">
      {/* Left-aligned column, as Google lays it out: the form keeps a readable
          width and the rest of the area stays empty rather than pushing the
          fields into the middle of a wide screen. */}
      <div className={`${isMobile ? 'px-4' : 'px-10'} py-6 space-y-5 w-full ${isWide ? 'max-w-6xl' : 'max-w-xl'}`}>
        {/* Actions ride on the form's own column, not across the whole area. */}
        <div className="flex items-center">
          <button onClick={onDone} aria-label={t('common_back')}
            className="w-10 h-10 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary">
            {isMobile ? <ArrowLeft size={22} /> : <X size={18} />}
          </button>
          <div className="flex-1" />
          {/* Favourite, before the contact even exists — the flag travels with
              the payload, so there is no "save then star" second step. */}
          <button type="button" onClick={() => f.setStarred(!f.starred)}
            title={f.starred ? t('ed_unstarred') : t('ed_starred')}
            aria-label={f.starred ? t('ed_unstarred') : t('ed_starred')}
            aria-pressed={f.starred}
            className="w-10 h-10 mr-1 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary">
            <Star size={20} className={f.starred ? 'text-yellow-500 fill-yellow-500' : ''} />
          </button>
          <Button size="sm" onClick={f.save} loading={f.saving}>{t('common_save')}</Button>
        </div>

        <div className="space-y-5">
          <AvatarPicker preview={previewContact} color={f.form.avatar_color}
            onPickFile={f.pickAvatar} onColor={c => f.setForm(v => ({ ...v, avatar_color: c }))} />

          <LabelPicker selected={f.labelIds} onChange={f.setLabelIds} />

          {/* Two columns split the form by MEANING, not by counting fields: who
              the person is on the left, how to reach them on the right. Narrow
              screens fall back to the single column, in the same reading order. */}
          <div className={isWide ? 'grid grid-cols-2 gap-x-12 gap-y-5 items-start' : 'space-y-5'}>
            <div className="space-y-5 min-w-0">
              <FieldGroup icon={<User size={20} />} fields={NAME_FIELDS} value={nameValues} primaryColor={ACCENT}
                onChange={v => f.setForm(prev => ({ ...prev, ...v }))} />

              <FieldGroup icon={<Building2 size={20} />} fields={ORG_FIELDS} value={orgValues} primaryColor={ACCENT}
                onChange={v => f.setForm(prev => ({ ...prev, ...v }))} />
            </div>

            <div className="space-y-5 min-w-0">
              <EmailSection rows={f.emails} onChange={f.setEmails} accent={ACCENT} />
              <PhoneSection rows={f.phones} onChange={f.setPhones} accent={ACCENT} />
              <AddressSection rows={f.addresses} onChange={f.setAddresses} accent={ACCENT} />
            </div>
          </div>

          {/* Birthday keeps its own always-visible row: it is the date people
              actually fill, and it drives the Birthdays smart view. */}
          <div className="flex items-center gap-3">
            <span className="w-5 flex-shrink-0 flex items-center justify-center text-text-secondary"><Cake size={20} /></span>
            {/* Day / month / year is a short row: let it keep its natural width
                instead of stretching the year box across a wide screen. */}
            <div className={`flex-1 min-w-0 ${isWide ? 'max-w-xl' : ''}`}>
              <DateField value={f.birthday} primaryColor={ACCENT} icon={null}
                onChange={(v: DateValue) => f.setBirthday(v)} />
            </div>
            <span className="w-8 flex-shrink-0" />
          </div>

          {showMore && (
            <MoreFields
              dates={f.dates} setDates={f.setDates}
              urls={f.urls} setUrls={f.setUrls}
              relations={f.relations} setRelations={f.setRelations}
              custom={f.customFields} setCustom={f.setCustom}
              accent={ACCENT} wide={isWide}
            />
          )}

          <div className="flex items-center gap-3">
            <span className="w-5 flex-shrink-0 flex items-center justify-center text-text-secondary"><FileText size={20} /></span>
            <div className="flex-1 min-w-0">
              <OutlinedField label={t('contacts_section_notes')} value={f.form.notes} multiline primaryColor={ACCENT}
                onChange={v => f.setForm(prev => ({ ...prev, notes: v }))} />
            </div>
            <span className="w-8 flex-shrink-0" />
          </div>

          <div className="pl-8">
            <button type="button" onClick={() => setShowMore(v => !v)}
              className="inline-flex items-center gap-1 text-sm text-primary hover:underline">
              {showMore ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              {showMore ? t('ed_less') : t('ed_more')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
