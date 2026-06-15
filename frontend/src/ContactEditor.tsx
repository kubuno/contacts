import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus, Trash2, X, Check } from 'lucide-react'
import { Dropdown, Button, DatePicker, Input as UiInput, Textarea } from '@ui'
import { Contact, ContactField, AddressField, contactsApi } from './api'
import { useContactsStore } from './store'
import ContactAvatar from './ContactAvatar'

interface Props {
  contact?: Contact
  onDone: () => void
}

const FIELD_TYPES_EMAIL  = ['work', 'home', 'other']
const FIELD_TYPES_PHONE  = ['mobile', 'work', 'home', 'fax', 'other']
const FIELD_TYPES_DATE   = ['birthday', 'anniversary', 'other']

const AVATAR_COLORS = [
  '#1a73e8', '#e53935', '#43a047', '#fb8c00', '#8e24aa',
  '#00acc1', '#f06292', '#6d4c41', '#546e7a', '#78909c',
]

function makeField(type = 'work'): ContactField { return { value: '', type } }

export default function ContactEditor({ contact, onDone }: Props) {
  const { t } = useTranslation('contacts')
  const { updateContact, fetchContacts } = useContactsStore()

  const [form, setForm] = useState({
    given_name:   contact?.given_name ?? '',
    family_name:  contact?.family_name ?? '',
    middle_name:  contact?.middle_name ?? '',
    nickname:     contact?.nickname ?? '',
    name_prefix:  contact?.name_prefix ?? '',
    name_suffix:  contact?.name_suffix ?? '',
    organization: contact?.organization ?? '',
    department:   contact?.department ?? '',
    job_title:    contact?.job_title ?? '',
    notes:        contact?.notes ?? '',
    avatar_color: contact?.avatar_color ?? '#1a73e8',
    emails:       contact?.emails.length ? [...contact.emails] : [makeField('work')],
    phones:       contact?.phones.length ? [...contact.phones] : [makeField('mobile')],
    addresses:    contact?.addresses.length ? [...contact.addresses] : [] as AddressField[],
    urls:         contact?.urls.length ? [...contact.urls] : [] as ContactField[],
    dates:        contact?.dates.length ? [...contact.dates] : [] as { label?: string; type: string; value: string }[],
  })

  const [saving, setSaving] = useState(false)
  const [avatarFile, setAvatarFile] = useState<File | null>(null)
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null)

  const displayName = [form.given_name, form.middle_name, form.family_name].filter(Boolean).join(' ')
    || form.nickname || form.organization || t('contacts_new_contact')

  const fakeContact = contact
    ? { ...contact, display_name: displayName, avatar_color: form.avatar_color, avatar_path: avatarPreview ?? contact.avatar_path }
    : { id: '', display_name: displayName, avatar_color: form.avatar_color, avatar_path: avatarPreview, emails: [], phones: [] } as unknown as Contact

  async function handleSave() {
    setSaving(true)
    try {
      const payload = {
        given_name:    form.given_name || null,
        family_name:   form.family_name || null,
        middle_name:   form.middle_name || null,
        nickname:      form.nickname || null,
        name_prefix:   form.name_prefix || null,
        name_suffix:   form.name_suffix || null,
        organization:  form.organization || null,
        department:    form.department || null,
        job_title:     form.job_title || null,
        notes:         form.notes || null,
        avatar_color:  form.avatar_color,
        emails:        form.emails.filter(e => e.value.trim()),
        phones:        form.phones.filter(p => p.value.trim()),
        addresses:     form.addresses.filter(a => Object.values(a).some(v => v && v !== a.type)),
        urls:          form.urls.filter(u => u.value.trim()),
        dates:         form.dates.filter(d => d.value.trim()),
      }

      let saved: Contact
      if (contact) {
        const res = await contactsApi.updateContact(contact.id, payload)
        saved = res.data.contact
        if (avatarFile) {
          await contactsApi.uploadAvatar(contact.id, avatarFile)
        }
        updateContact(saved)
      } else {
        const res = await contactsApi.createContact(payload)
        saved = res.data.contact
        if (avatarFile) {
          await contactsApi.uploadAvatar(saved.id, avatarFile)
        }
        await fetchContacts()
      }
      onDone()
    } finally {
      setSaving(false)
    }
  }

  function handleAvatarChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setAvatarFile(file)
    const reader = new FileReader()
    reader.onload = () => setAvatarPreview(reader.result as string)
    reader.readAsDataURL(file)
  }

  return (
    <div className="flex-1 overflow-y-auto bg-white">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-3 border-b border-border bg-surface-1">
        <button onClick={onDone} className="p-1.5 rounded hover:bg-surface-2 text-text-secondary">
          <X size={16} />
        </button>
        <span className="flex-1 text-sm font-medium text-text-primary">
          {contact ? t('contacts_edit_contact') : t('contacts_new_contact')}
        </span>
        <Button
          size="sm"
          icon={<Check size={14} />}
          onClick={handleSave}
          loading={saving}
        >
          {t('common_save')}
        </Button>
      </div>

      <div className="p-6 space-y-6 max-w-xl">
        {/* Avatar + color */}
        <div className="flex items-center gap-4">
          <label className="cursor-pointer relative group">
            <ContactAvatar contact={fakeContact} size="xl" />
            <div className="absolute inset-0 rounded-full bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
              <span className="text-white text-xs">{t('contacts_photo')}</span>
            </div>
            <input type="file" accept="image/*" className="hidden" onChange={handleAvatarChange} />
          </label>
          <div>
            <p className="text-xs text-text-secondary mb-2">{t('contacts_color')}</p>
            <div className="flex flex-wrap gap-1.5">
              {AVATAR_COLORS.map(c => (
                <button
                  key={c}
                  className={`w-6 h-6 rounded-full border-2 transition-transform ${form.avatar_color === c ? 'border-text-primary scale-110' : 'border-transparent'}`}
                  style={{ backgroundColor: c }}
                  onClick={() => setForm(f => ({ ...f, avatar_color: c }))}
                />
              ))}
            </div>
          </div>
        </div>

        {/* Name */}
        <Section title={t('contacts_section_name')}>
          <div className="grid grid-cols-2 gap-2">
            <Input label={t('contacts_field_given_name')} value={form.given_name} onChange={v => setForm(f => ({ ...f, given_name: v }))} />
            <Input label={t('contacts_field_family_name')} value={form.family_name} onChange={v => setForm(f => ({ ...f, family_name: v }))} />
            <Input label={t('contacts_field_middle_name')} value={form.middle_name} onChange={v => setForm(f => ({ ...f, middle_name: v }))} />
            <Input label={t('contacts_field_nickname')} value={form.nickname} onChange={v => setForm(f => ({ ...f, nickname: v }))} />
            <Input label={t('contacts_field_prefix')} value={form.name_prefix} onChange={v => setForm(f => ({ ...f, name_prefix: v }))} />
            <Input label={t('contacts_field_suffix')} value={form.name_suffix} onChange={v => setForm(f => ({ ...f, name_suffix: v }))} />
          </div>
        </Section>

        {/* Organization */}
        <Section title={t('contacts_section_org')}>
          <div className="grid grid-cols-2 gap-2">
            <Input label={t('contacts_field_company')} value={form.organization} onChange={v => setForm(f => ({ ...f, organization: v }))} />
            <Input label={t('contacts_field_department')} value={form.department} onChange={v => setForm(f => ({ ...f, department: v }))} />
            <Input label={t('contacts_field_job_title')} value={form.job_title} onChange={v => setForm(f => ({ ...f, job_title: v }))} className="col-span-2" />
          </div>
        </Section>

        {/* Emails */}
        <MultiFieldSection
          title={t('contacts_section_emails')} items={form.emails} types={FIELD_TYPES_EMAIL}
          renderValue={(f, onChange) => <div className="flex-1 min-w-0"><UiInput type="email" value={f.value} placeholder="email@example.com" onChange={e => onChange({ ...f, value: e.target.value })} /></div>}
          onAdd={() => setForm(f => ({ ...f, emails: [...f.emails, makeField('work')] }))}
          onChange={v => setForm(f => ({ ...f, emails: v }))}
          onRemove={i => setForm(f => ({ ...f, emails: f.emails.filter((_, j) => j !== i) }))}
        />

        {/* Phones */}
        <MultiFieldSection
          title={t('contacts_section_phones')} items={form.phones} types={FIELD_TYPES_PHONE}
          renderValue={(f, onChange) => <div className="flex-1 min-w-0"><UiInput type="tel" value={f.value} placeholder="+33 6 00 00 00 00" onChange={e => onChange({ ...f, value: e.target.value })} /></div>}
          onAdd={() => setForm(f => ({ ...f, phones: [...f.phones, makeField('mobile')] }))}
          onChange={v => setForm(f => ({ ...f, phones: v }))}
          onRemove={i => setForm(f => ({ ...f, phones: f.phones.filter((_, j) => j !== i) }))}
        />

        {/* Dates */}
        <MultiFieldSection
          title={t('contacts_section_dates')} items={form.dates as ContactField[]} types={FIELD_TYPES_DATE}
          renderValue={(f, onChange) => <DatePicker mode="date" value={f.value || null} onChange={v => onChange({ ...f, value: v ?? '' })} />}
          onAdd={() => setForm(f => ({ ...f, dates: [...f.dates, { type: 'birthday', value: '' }] }))}
          onChange={v => setForm(f => ({ ...f, dates: v }))}
          onRemove={i => setForm(f => ({ ...f, dates: f.dates.filter((_, j) => j !== i) }))}
        />

        {/* Notes */}
        <Section title={t('contacts_section_notes')}>
          <Textarea
            value={form.notes}
            onChange={e => setForm(f => ({ ...f, notes: e.target.value }))}
            rows={3}
            className="h-auto min-h-0"
            placeholder={t('contacts_notes_placeholder')}
          />
        </Section>
      </div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-text-secondary uppercase tracking-wide mb-2">{title}</h3>
      {children}
    </div>
  )
}

function Input({ label, value, onChange, className = '' }: { label: string; value: string; onChange: (v: string) => void; className?: string }) {
  return (
    <div className={className}>
      <label className="block text-xs text-text-secondary mb-1">{label}</label>
      <UiInput
        type="text"
        value={value}
        onChange={e => onChange(e.target.value)}
      />
    </div>
  )
}

function MultiFieldSection<T extends ContactField>({
  title, items, types, renderValue, onAdd, onChange, onRemove,
}: {
  title: string
  items: T[]
  types: string[]
  renderValue: (item: T, onChange: (v: T) => void) => React.ReactNode
  onAdd: () => void
  onChange: (items: T[]) => void
  onRemove: (i: number) => void
}) {
  const { t } = useTranslation('contacts')
  return (
    <Section title={title}>
      <div className="space-y-2">
        {items.map((item, i) => (
          <div key={i} className="flex items-center gap-2">
            <Dropdown
              width={96}
              value={item.type}
              onChange={v => onChange(items.map((x, j) => j === i ? { ...x, type: v } : x))}
              options={types.map(type => ({ value: type, label: t(`contacts_type_${type}`) }))}
            />
            {renderValue(item, v => onChange(items.map((x, j) => j === i ? v : x)))}
            <button onClick={() => onRemove(i)} className="text-text-secondary hover:text-danger flex-shrink-0">
              <Trash2 size={14} />
            </button>
          </div>
        ))}
        <button onClick={onAdd} className="flex items-center gap-1 text-xs text-primary hover:text-primary-hover">
          <Plus size={12} /> {t('contacts_add')}
        </button>
      </div>
    </Section>
  )
}
