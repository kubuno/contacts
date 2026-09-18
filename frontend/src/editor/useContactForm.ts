import { useState } from 'react'
import type { DateValue, PhoneValue } from '@ui'
import { Contact, AddressField, CustomField, contactsApi } from '../api'
import { useContactsStore } from '../store'
import { parseDateValue, formatDateValue, EMPTY_DATE } from './dateValue'
import { parsePhoneValue, formatPhoneValue, isPhoneEmpty, DEFAULT_COUNTRY } from './phoneValue'

/* Editor state and saving, kept out of the view so the form's shape is readable
 * on its own. Composite fields (phone, date) live here in their STRUCTURED form
 * — the shape their @ui component speaks — and are folded back into the strings
 * the API stores only at save time. */

export interface EmailRow  { value: string; type: string }
export interface PhoneRow  { phone: PhoneValue }
export interface DateRow   { date: DateValue }
export interface UrlRow    { value: string; type: string }
export interface RelRow    { value: string; type: string }

const BIRTHDAY = 'birthday'

export function useContactForm(contact: Contact | undefined, onDone: () => void) {
  const { updateContact, fetchContacts, fetchLabels } = useContactsStore()

  const birthdaySrc = contact?.dates?.find(d => d.type === BIRTHDAY)
  const otherDatesSrc = (contact?.dates ?? []).filter(d => d.type !== BIRTHDAY)

  const [form, setForm] = useState({
    // Name — the two visible fields plus the ones folded behind the chevron.
    given_name:   contact?.given_name ?? '',
    family_name:  contact?.family_name ?? '',
    middle_name:  contact?.middle_name ?? '',
    nickname:     contact?.nickname ?? '',
    name_prefix:  contact?.name_prefix ?? '',
    name_suffix:  contact?.name_suffix ?? '',
    pronouns:     contact?.pronouns ?? '',
    // Organisation
    organization: contact?.organization ?? '',
    job_title:    contact?.job_title ?? '',
    department:   contact?.department ?? '',
    notes:        contact?.notes ?? '',
    avatar_color: contact?.avatar_color ?? '#1a73e8',
  })

  // A new contact opens with one empty e-mail and one empty phone, like Google.
  const [emails, setEmails] = useState<EmailRow[]>(
    contact?.emails?.length ? contact.emails.map(e => ({ value: e.value, type: e.type || 'work' }))
                            : [{ value: '', type: 'work' }])
  const [phones, setPhones] = useState<PhoneRow[]>(
    contact?.phones?.length ? contact.phones.map(p => ({ phone: parsePhoneValue(p.value, p.type) }))
                            : [{ phone: { country: DEFAULT_COUNTRY, number: '', label: '' } }])
  const [addresses, setAddresses] = useState<AddressField[]>(contact?.addresses ? [...contact.addresses] : [])
  const [birthday, setBirthday]   = useState<DateValue>(parseDateValue(birthdaySrc?.value))
  const [dates, setDates]         = useState<DateRow[]>(otherDatesSrc.map(d => ({ date: { ...parseDateValue(d.value), label: d.type } })))
  const [urls, setUrls]           = useState<UrlRow[]>((contact?.urls ?? []).map(u => ({ value: u.value, type: u.type || 'work' })))
  const [relations, setRelations] = useState<RelRow[]>((contact?.relations ?? []).map(r => ({ value: r.value, type: r.type || 'other' })))
  const [customFields, setCustom] = useState<CustomField[]>(contact?.custom_fields ? [...contact.custom_fields] : [])
  const [labelIds, setLabelIds]   = useState<string[]>(contact?.label_ids ? [...contact.label_ids] : [])
  // Favourite, settable before the contact even exists: both DTOs accept it, so
  // it rides in the payload rather than needing a second round-trip.
  const [starred, setStarred]     = useState<boolean>(contact?.is_starred ?? false)

  const [saving, setSaving] = useState(false)
  const [avatarFile, setAvatarFile] = useState<File | null>(null)
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null)

  const displayName = [form.given_name, form.middle_name, form.family_name].filter(Boolean).join(' ')
    || form.nickname || form.organization || ''

  function pickAvatar(file: File | null) {
    if (!file) return
    setAvatarFile(file)
    const reader = new FileReader()
    reader.onload = () => setAvatarPreview(reader.result as string)
    reader.readAsDataURL(file)
  }

  async function save() {
    setSaving(true)
    try {
      // A date is kept only once it FORMATS — a day without its month yields no
      // usable value, and sending `null` is what the API rejects outright. The
      // emptiness check alone is not enough: a half-filled date is not empty.
      const bday = formatDateValue(birthday)
      const allDates = [
        ...(bday ? [{ type: BIRTHDAY, value: bday }] : []),
        ...dates
          .map(d => ({ type: d.date.label?.trim() || 'other', value: formatDateValue(d.date) }))
          .filter((d): d is { type: string; value: string } => d.value !== null),
      ]
      const payload = {
        given_name:   form.given_name  || null,
        family_name:  form.family_name || null,
        middle_name:  form.middle_name || null,
        nickname:     form.nickname    || null,
        name_prefix:  form.name_prefix || null,
        name_suffix:  form.name_suffix || null,
        pronouns:     form.pronouns    || null,
        organization: form.organization || null,
        department:   form.department   || null,
        job_title:    form.job_title    || null,
        notes:        form.notes || null,
        avatar_color: form.avatar_color,
        is_starred:   starred,
        emails:    emails.filter(e => e.value.trim()).map(e => ({ value: e.value.trim(), type: e.type })),
        phones:    phones.filter(p => !isPhoneEmpty(p.phone))
                         .map(p => ({ value: formatPhoneValue(p.phone), type: p.phone.label?.trim() || 'mobile' })),
        addresses: addresses.filter(a => Object.entries(a).some(([k, v]) => k !== 'type' && v)),
        urls:      urls.filter(u => u.value.trim()).map(u => ({ value: u.value.trim(), type: u.type })),
        relations: relations.filter(r => r.value.trim()).map(r => ({ value: r.value.trim(), type: r.type })),
        custom_fields: customFields.filter(c => c.label.trim() && c.value.trim()),
        dates: allDates,
      }

      let saved: Contact
      if (contact) {
        saved = (await contactsApi.updateContact(contact.id, payload)).data.contact
        if (avatarFile) await contactsApi.uploadAvatar(contact.id, avatarFile)
        updateContact(saved)
      } else {
        saved = (await contactsApi.createContact(payload)).data.contact
        if (avatarFile) await contactsApi.uploadAvatar(saved.id, avatarFile)
      }

      // Labels are membership, not a column: diff them against what the contact
      // had. A new contact has no previous set, so everything picked is added.
      const before = new Set(contact?.label_ids ?? [])
      const after  = new Set(labelIds)
      const added   = [...after].filter(id => !before.has(id))
      const removed = [...before].filter(id => !after.has(id))
      if (added.length || removed.length) {
        await Promise.all([
          ...added.map(id => contactsApi.addLabelMembers(id, [saved.id])),
          ...removed.map(id => contactsApi.removeLabelMembers(id, [saved.id])),
        ])
        fetchLabels()
      }

      await fetchContacts()
      onDone()
    } finally {
      setSaving(false)
    }
  }

  return {
    form, setForm,
    emails, setEmails, phones, setPhones, addresses, setAddresses,
    birthday, setBirthday, dates, setDates, urls, setUrls,
    relations, setRelations, customFields, setCustom,
    labelIds, setLabelIds,
    starred, setStarred,
    saving, save,
    avatarPreview, pickAvatar,
    displayName,
    EMPTY_DATE,
  }
}
