import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation } from '@tanstack/react-query'
import {
  Mail, Phone, MapPin, Globe, Calendar, Users, Star, StarOff,
  Trash2, Edit2, Download, RefreshCw,
} from 'lucide-react'
import { Contact, contactsApi } from './api'
import { useContactsStore } from './store'
import ContactAvatar from './ContactAvatar'
import ContactEditor from './ContactEditor'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog } from '@ui'

interface Props {
  contact: Contact
}

export default function ContactDetail({ contact }: Props) {
  const { t } = useTranslation('contacts')
  const [editing, setEditing] = useState(false)
  const { removeContact, updateContact, view } = useContactsStore()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const star = useMutation({
    mutationFn: () => contact.is_starred
      ? contactsApi.unstarContact(contact.id)
      : contactsApi.starContact(contact.id),
    onSuccess: () => {
      updateContact({ ...contact, is_starred: !contact.is_starred })
    },
  })

  const trash = useMutation({
    mutationFn: () => contactsApi.trashContact(contact.id),
    onSuccess: () => removeContact(contact.id),
  })

  const restore = useMutation({
    mutationFn: () => contactsApi.restoreContact(contact.id),
    onSuccess: () => removeContact(contact.id),
  })

  const deletePerm = useMutation({
    mutationFn: () => contactsApi.deleteContact(contact.id),
    onSuccess: () => removeContact(contact.id),
  })

  if (editing) {
    return <ContactEditor contact={contact} onDone={() => setEditing(false)} />
  }

  return (
    <div className="flex-1 overflow-y-auto bg-white">
      {/* Header */}
      <div className="bg-surface-1 border-b border-border px-6 py-5">
        <div className="flex items-start gap-4">
          <ContactAvatar contact={contact} size="xl" />
          <div className="flex-1 min-w-0">
            <h2 className="text-xl font-semibold text-text-primary">{contact.display_name || t('no_name')}</h2>
            {contact.job_title && <p className="text-sm text-text-secondary mt-0.5">{contact.job_title}</p>}
            {contact.organization && <p className="text-sm text-text-secondary">{contact.organization}</p>}
          </div>
          <div className="flex items-center gap-1 flex-shrink-0">
            <ActionBtn onClick={() => setEditing(true)} title={t('common_edit')}>
              <Edit2 size={16} />
            </ActionBtn>
            <ActionBtn onClick={() => star.mutate()} title={contact.is_starred ? t('contacts_unstar') : t('contacts_star')}>
              {contact.is_starred
                ? <Star size={16} className="text-yellow-500 fill-yellow-500" />
                : <StarOff size={16} />}
            </ActionBtn>
            {view === 'trashed' ? (
              <>
                <ActionBtn onClick={() => restore.mutate()} title={t('contacts_restore')}>
                  <RefreshCw size={16} />
                </ActionBtn>
                <ActionBtn onClick={async () => {
                  const ok = await confirm({ title: t('contacts_delete_perm_title'), message: t('contacts_delete_perm_msg'), confirmLabel: t('common_delete'), variant: 'danger' })
                  if (ok) deletePerm.mutate()
                }} title={t('contacts_delete_perm')} danger>
                  <Trash2 size={16} />
                </ActionBtn>
              </>
            ) : (
              <ActionBtn onClick={() => trash.mutate()} title={t('contacts_move_to_trash')}>
                <Trash2 size={16} />
              </ActionBtn>
            )}
            <ActionBtn onClick={() => contactsApi.exportVcf({ })} title={t('common_export')}>
              <Download size={16} />
            </ActionBtn>
          </div>
        </div>
      </div>

      {/* Fields */}
      <div className="px-6 py-4 space-y-4">
        <Section title={t('contacts_section_contact_info')}>
          {contact.emails.map((e, i) => (
            <FieldRow key={i} icon={<Mail size={15} />} label={e.type || t('contacts_field_email')}>
              <a href={`mailto:${e.value}`} className="text-primary hover:underline text-sm">{e.value}</a>
            </FieldRow>
          ))}
          {contact.phones.map((p, i) => (
            <FieldRow key={i} icon={<Phone size={15} />} label={p.type || t('contacts_field_phone')}>
              <a href={`tel:${p.value}`} className="text-primary hover:underline text-sm">{p.value}</a>
            </FieldRow>
          ))}
          {contact.addresses.map((a, i) => (
            <FieldRow key={i} icon={<MapPin size={15} />} label={a.type || t('contacts_field_address')}>
              <p className="text-sm text-text-primary whitespace-pre-line">
                {[a.street, a.city, a.region, a.postcode, a.country].filter(Boolean).join(', ')}
              </p>
            </FieldRow>
          ))}
          {contact.urls.map((u, i) => (
            <FieldRow key={i} icon={<Globe size={15} />} label={u.type || t('contacts_field_website')}>
              <a href={u.value} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline text-sm truncate">{u.value}</a>
            </FieldRow>
          ))}
        </Section>

        {contact.dates.length > 0 && (
          <Section title={t('contacts_section_dates')}>
            {contact.dates.map((d, i) => (
              <FieldRow key={i} icon={<Calendar size={15} />} label={d.type || t('contacts_field_date')}>
                <span className="text-sm text-text-primary">{d.value}</span>
              </FieldRow>
            ))}
          </Section>
        )}

        {contact.relations.length > 0 && (
          <Section title={t('contacts_section_relations')}>
            {contact.relations.map((r, i) => (
              <FieldRow key={i} icon={<Users size={15} />} label={r.type || t('contacts_field_relation')}>
                <span className="text-sm text-text-primary">{r.value}</span>
              </FieldRow>
            ))}
          </Section>
        )}

        {contact.notes && (
          <Section title={t('contacts_section_notes')}>
            <p className="text-sm text-text-primary whitespace-pre-wrap px-2">{contact.notes}</p>
          </Section>
        )}
      </div>
      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-text-secondary uppercase tracking-wide mb-2">{title}</h3>
      <div className="space-y-1">{children}</div>
    </div>
  )
}

function FieldRow({ icon, label, children }: { icon: React.ReactNode; label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-3 py-1">
      <span className="text-text-secondary mt-0.5 flex-shrink-0">{icon}</span>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-text-secondary capitalize">{label}</p>
        {children}
      </div>
    </div>
  )
}

function ActionBtn({ onClick, title, children, danger }: {
  onClick: () => void; title: string; children: React.ReactNode; danger?: boolean
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={`p-2 rounded-lg transition-colors ${danger ? 'hover:bg-danger-light hover:text-danger text-text-secondary' : 'hover:bg-surface-2 text-text-secondary hover:text-text-primary'}`}
    >
      {children}
    </button>
  )
}
