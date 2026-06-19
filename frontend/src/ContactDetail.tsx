import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation } from '@tanstack/react-query'
import {
  Mail, Phone, MapPin, Globe, Calendar, Users, Star, StarOff, Trash2, Edit2,
  Download, RefreshCw, Copy, Check, Archive, ArchiveRestore, Ban, Share2,
  MessageSquare, CalendarPlus, History, Clock, Plus, Tag,
} from 'lucide-react'
import { MenuDropdown, type MenuItem, type MenuDropdownPos } from '@ui'
import { Contact, contactsApi, type Interaction, type ChangeEntry } from './api'
import { useContactsStore } from './store'
import ContactAvatar from './ContactAvatar'
import ContactEditor from './ContactEditor'
import ShareModal from './ShareModal'
import { LabelChip } from './widgets'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog } from '@ui'

export default function ContactDetail({ contact }: { contact: Contact }) {
  const { t } = useTranslation('contacts')
  const [editing, setEditing] = useState(false)
  const [sharing, setSharing] = useState(false)
  const [interactions, setInteractions] = useState<Interaction[]>([])
  const [history, setHistory] = useState<ChangeEntry[]>([])
  const [showHistory, setShowHistory] = useState(false)
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)
  const [copiedKey, setCopiedKey] = useState<string | null>(null)
  const { removeContact, updateContact, view, labels, fetchContacts, fetchLabels } = useContactsStore()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  useEffect(() => {
    contactsApi.listInteractions(contact.id).then(r => setInteractions(r.data.interactions))
    contactsApi.getHistory(contact.id).then(r => setHistory(r.data.history))
  }, [contact.id])

  function copy(text: string, key: string) {
    navigator.clipboard?.writeText(text).then(() => { setCopiedKey(key); setTimeout(() => setCopiedKey(null), 1500) })
  }

  const star = useMutation({
    mutationFn: () => contact.is_starred ? contactsApi.unstarContact(contact.id) : contactsApi.starContact(contact.id),
    onSuccess: () => updateContact({ ...contact, is_starred: !contact.is_starred }),
  })
  const trash = useMutation({ mutationFn: () => contactsApi.trashContact(contact.id), onSuccess: () => removeContact(contact.id) })
  const restore = useMutation({ mutationFn: () => contactsApi.restoreContact(contact.id), onSuccess: () => removeContact(contact.id) })
  const deletePerm = useMutation({ mutationFn: () => contactsApi.deleteContact(contact.id), onSuccess: () => removeContact(contact.id) })
  const archive = useMutation({
    mutationFn: () => contact.is_archived ? contactsApi.unarchiveContact(contact.id) : contactsApi.archiveContact(contact.id),
    onSuccess: () => { fetchContacts() },
  })
  const block = useMutation({
    mutationFn: () => contact.is_blocked ? contactsApi.unblockContact(contact.id) : contactsApi.blockContact(contact.id),
    onSuccess: () => updateContact({ ...contact, is_blocked: !contact.is_blocked }),
  })

  async function toggleLabel(labelId: string) {
    const has = contact.label_ids?.includes(labelId)
    if (has) await contactsApi.removeLabelMembers(labelId, [contact.id])
    else await contactsApi.addLabelMembers(labelId, [contact.id])
    const next = has ? (contact.label_ids ?? []).filter(i => i !== labelId) : [...(contact.label_ids ?? []), labelId]
    updateContact({ ...contact, label_ids: next })
    fetchLabels()
  }
  function openLabelMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: labels.map(l => ({
      type: 'action', label: l.name, checked: contact.label_ids?.includes(l.id),
      icon: <span className="w-3 h-3 rounded-full inline-block" style={{ backgroundColor: l.color }} />,
      onClick: () => toggleLabel(l.id),
    })) })
  }
  async function addInteraction(type: string) {
    await contactsApi.addInteraction(contact.id, type)
    const r = await contactsApi.listInteractions(contact.id); setInteractions(r.data.interactions)
  }
  function openInteractionMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const it = (type: string, label: string): MenuItem => ({ type: 'action', label, onClick: () => addInteraction(type) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: [
      it('call', t('inter_call')), it('email', t('inter_email')), it('meeting', t('inter_meeting')), it('note', t('inter_note')),
    ]})
  }

  if (editing) return <ContactEditor contact={contact} onDone={() => { setEditing(false); fetchContacts() }} />

  const myLabels = labels.filter(l => contact.label_ids?.includes(l.id))
  const firstEmail = contact.emails[0]?.value
  const firstPhone = contact.phones[0]?.value

  return (
    <div className="flex-1 overflow-y-auto bg-white">
      {/* Header */}
      <div className="bg-surface-1 border-b border-border px-6 py-5">
        <div className="flex items-start gap-4">
          <ContactAvatar contact={contact} size="xl" />
          <div className="flex-1 min-w-0">
            <h2 className="text-xl font-semibold text-text-primary flex items-center gap-2">
              {contact.display_name || t('no_name')}
              {contact.is_blocked && <Ban size={15} className="text-danger" />}
            </h2>
            {contact.pronouns && <p className="text-xs text-text-tertiary">{contact.pronouns}</p>}
            {contact.job_title && <p className="text-sm text-text-secondary mt-0.5">{contact.job_title}</p>}
            {contact.organization && <p className="text-sm text-text-secondary">{contact.organization}</p>}
          </div>
          <div className="flex items-center gap-1 flex-shrink-0">
            <IconBtn onClick={() => setEditing(true)} title={t('common_edit')}><Edit2 size={16} /></IconBtn>
            <IconBtn onClick={() => star.mutate()} title={contact.is_starred ? t('contacts_unstar') : t('contacts_star')}>
              {contact.is_starred ? <Star size={16} className="text-yellow-500 fill-yellow-500" /> : <StarOff size={16} />}
            </IconBtn>
            <IconBtn onClick={() => setSharing(true)} title={t('det_share')}><Share2 size={16} /></IconBtn>
            {view === 'trashed' ? (
              <>
                <IconBtn onClick={() => restore.mutate()} title={t('contacts_restore')}><RefreshCw size={16} /></IconBtn>
                <IconBtn danger onClick={async () => { if (await confirm({ title: t('contacts_delete_perm_title'), message: t('contacts_delete_perm_msg'), confirmLabel: t('common_delete'), variant: 'danger' })) deletePerm.mutate() }} title={t('contacts_delete_perm')}><Trash2 size={16} /></IconBtn>
              </>
            ) : (
              <IconBtn onClick={() => trash.mutate()} title={t('contacts_move_to_trash')}><Trash2 size={16} /></IconBtn>
            )}
          </div>
        </div>

        {/* Quick actions */}
        <div className="flex items-center gap-2 mt-4">
          <QuickAction icon={<Mail size={16} />} label={t('det_email_action')} disabled={!firstEmail} onClick={() => firstEmail && (window.location.href = `mailto:${firstEmail}`)} />
          <QuickAction icon={<Phone size={16} />} label={t('det_call')} disabled={!firstPhone} onClick={() => firstPhone && (window.location.href = `tel:${firstPhone}`)} />
          <QuickAction icon={<MessageSquare size={16} />} label={t('det_chat')} onClick={() => { window.location.href = '/chat' }} />
          <QuickAction icon={<CalendarPlus size={16} />} label={t('det_calendar')} onClick={() => { window.location.href = '/calendar' }} />
        </div>
      </div>

      <div className="px-6 py-4 space-y-4">
        {/* Labels */}
        <div className="flex items-center gap-2 flex-wrap">
          {myLabels.map(l => <LabelChip key={l.id} name={l.name} color={l.color} onRemove={() => toggleLabel(l.id)} />)}
          <button onClick={openLabelMenu} className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border border-dashed border-border text-text-secondary hover:bg-surface-1">
            <Tag size={11} />{t('label_add')}
          </button>
        </div>

        <Section title={t('contacts_section_contact_info')}>
          {contact.emails.map((e, i) => (
            <FieldRow key={`e${i}`} icon={<Mail size={15} />} label={e.type || t('contacts_field_email')}
              action={<CopyBtn onClick={() => copy(e.value, `e${i}`)} copied={copiedKey === `e${i}`} />}>
              <a href={`mailto:${e.value}`} className="text-primary hover:underline text-sm">{e.value}</a>
            </FieldRow>
          ))}
          {contact.phones.map((p, i) => (
            <FieldRow key={`p${i}`} icon={<Phone size={15} />} label={p.type || t('contacts_field_phone')}
              action={<CopyBtn onClick={() => copy(p.value, `p${i}`)} copied={copiedKey === `p${i}`} />}>
              <a href={`tel:${p.value}`} className="text-primary hover:underline text-sm">{p.value}</a>
            </FieldRow>
          ))}
          {contact.addresses.map((a, i) => {
            const full = [a.street, a.city, a.region, a.postcode, a.country].filter(Boolean).join(', ')
            return (
              <FieldRow key={`a${i}`} icon={<MapPin size={15} />} label={a.type || t('contacts_field_address')}>
                <p className="text-sm text-text-primary">{full}</p>
                {full && <a href={`https://www.openstreetmap.org/search?query=${encodeURIComponent(full)}`} target="_blank" rel="noopener noreferrer" className="text-xs text-primary hover:underline">{t('det_map')}</a>}
              </FieldRow>
            )
          })}
          {contact.urls.map((u, i) => (
            <FieldRow key={`u${i}`} icon={<Globe size={15} />} label={u.type || t('contacts_field_website')}>
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

        {/* Interactions */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-xs font-semibold text-text-secondary uppercase tracking-wide">{t('det_interactions')}</h3>
            <button onClick={openInteractionMenu} className="p-1 rounded-full hover:bg-surface-2 text-text-secondary" title={t('det_add_interaction')}><Plus size={14} /></button>
          </div>
          {contact.last_interaction_at && (
            <p className="text-xs text-text-secondary mb-2 px-2">{t('det_last_contact')} : {new Date(contact.last_interaction_at).toLocaleDateString()}</p>
          )}
          {!interactions.length ? (
            <p className="text-sm text-text-tertiary px-2">{t('det_no_interactions')}</p>
          ) : (
            <div className="space-y-1.5">
              {interactions.slice(0, 8).map(it => (
                <div key={it.id} className="flex items-start gap-2 px-2">
                  <Clock size={13} className="text-text-secondary mt-0.5" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-text-primary capitalize">{it.interaction_type}{it.summary ? ` — ${it.summary}` : ''}</p>
                    <p className="text-xs text-text-secondary">{new Date(it.occurred_at).toLocaleString()}</p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* History */}
        {history.length > 0 && (
          <div>
            <button onClick={() => setShowHistory(v => !v)} className="flex items-center gap-1.5 text-xs font-semibold text-text-secondary uppercase tracking-wide hover:text-text-primary">
              <History size={13} />{t('det_history')} ({history.length})
            </button>
            {showHistory && (
              <div className="mt-2 space-y-1">
                {history.slice(0, 12).map((h, i) => (
                  <p key={i} className="text-xs text-text-secondary px-2">
                    {t('det_changed', { field: h.field })}: <span className="line-through opacity-60">{h.old_value || '∅'}</span> → {h.new_value || '∅'}
                    <span className="opacity-50 ml-1">({new Date(h.changed_at).toLocaleDateString()})</span>
                  </p>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Manage */}
        {view !== 'trashed' && (
          <div className="flex items-center gap-2 pt-2 border-t border-border">
            <button onClick={() => archive.mutate()} className="flex items-center gap-1.5 text-xs text-text-secondary hover:text-text-primary px-2 py-1">
              {contact.is_archived ? <ArchiveRestore size={14} /> : <Archive size={14} />}{contact.is_archived ? t('det_unarchive') : t('det_archive')}
            </button>
            <button onClick={() => block.mutate()} className="flex items-center gap-1.5 text-xs text-text-secondary hover:text-danger px-2 py-1">
              <Ban size={14} />{contact.is_blocked ? t('det_unblock') : t('det_block')}
            </button>
            <button onClick={() => contactsApi.exportVcf({})} className="flex items-center gap-1.5 text-xs text-text-secondary hover:text-text-primary px-2 py-1">
              <Download size={14} />{t('common_export')}
            </button>
          </div>
        )}
      </div>

      {sharing && <ShareModal contactId={contact.id} onClose={() => setSharing(false)} />}
      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
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

function FieldRow({ icon, label, action, children }: { icon: React.ReactNode; label: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-3 py-1 group">
      <span className="text-text-secondary mt-0.5 flex-shrink-0">{icon}</span>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-text-secondary capitalize">{label}</p>
        {children}
      </div>
      {action && <span className="opacity-0 group-hover:opacity-100 transition-opacity">{action}</span>}
    </div>
  )
}

function CopyBtn({ onClick, copied }: { onClick: () => void; copied: boolean }) {
  return (
    <button onClick={onClick} className="p-1 rounded hover:bg-surface-2 text-text-secondary">
      {copied ? <Check size={13} className="text-success" /> : <Copy size={13} />}
    </button>
  )
}

function QuickAction({ icon, label, onClick, disabled }: { icon: React.ReactNode; label: string; onClick: () => void; disabled?: boolean }) {
  return (
    <button onClick={onClick} disabled={disabled} title={label}
      className={`flex flex-col items-center gap-1 px-3 py-1.5 rounded-lg text-xs transition-colors ${disabled ? 'opacity-40 cursor-not-allowed text-text-secondary' : 'text-primary hover:bg-primary-light'}`}>
      {icon}<span>{label}</span>
    </button>
  )
}

function IconBtn({ onClick, title, children, danger }: { onClick: () => void; title: string; children: React.ReactNode; danger?: boolean }) {
  return (
    <button onClick={onClick} title={title}
      className={`p-2 rounded-lg transition-colors ${danger ? 'hover:bg-danger-light hover:text-danger text-text-secondary' : 'hover:bg-surface-2 text-text-secondary hover:text-text-primary'}`}>
      {children}
    </button>
  )
}
