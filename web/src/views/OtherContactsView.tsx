import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Inbox, UserPlus, X, Mail, Phone, AtSign } from 'lucide-react'
import { Spinner } from '@ui'
import { contactsApi, type OtherContact } from '../api'
import { useContactsStore } from '../store'

/**
 * "Other contacts": people the user has actually dealt with — a mail
 * correspondent, someone met in a chat — who were never saved into the address
 * book. The list is fed by the modules that handle interlocutors; contacts only
 * keeps the memory and offers the two useful verdicts: keep, or stop suggesting.
 *
 * Anyone already in the address book is filtered out server-side, so saving one
 * makes it leave the list on its own.
 */
export default function OtherContactsView() {
  const { t } = useTranslation('contacts')
  const { fetchContacts } = useContactsStore()
  const [items, setItems] = useState<OtherContact[] | null>(null)
  const [busy, setBusy] = useState<string | null>(null)

  const load = () => contactsApi.listOtherContacts()
    .then(r => setItems(r.data.other_contacts))
    .catch(() => setItems([]))
  useEffect(() => { void load() }, [])

  async function save(o: OtherContact) {
    setBusy(o.id)
    try { await contactsApi.saveOtherContact(o.id); await load(); fetchContacts() }
    finally { setBusy(null) }
  }
  async function dismiss(o: OtherContact) {
    setBusy(o.id)
    try { await contactsApi.dismissOtherContact(o.id); await load() }
    finally { setBusy(null) }
  }

  const icon = (kind: string) =>
    kind === 'phone' ? <Phone size={18} /> : kind === 'email' ? <Mail size={18} /> : <AtSign size={18} />

  if (!items) return <div className="flex-1 flex justify-center py-16"><Spinner /></div>

  return (
    <div className="flex-1 overflow-y-auto px-6 py-6">
      <div className="max-w-2xl">
        <h1 className="text-2xl font-normal text-text-primary mb-1">{t('other_contacts')}</h1>
        <p className="text-sm text-text-secondary mb-6">{t('other_contacts_help')}</p>

        {!items.length ? (
          <div className="flex flex-col items-center gap-3 py-16 text-center">
            <Inbox size={40} className="text-text-tertiary" />
            <p className="text-sm text-text-secondary">{t('other_contacts_empty')}</p>
          </div>
        ) : (
          <ul className="divide-y divide-border">
            {items.map(o => (
              <li key={o.id} className="group flex items-center gap-3 py-3">
                <span className="text-text-secondary flex-shrink-0">{icon(o.kind)}</span>
                <div className="flex-1 min-w-0">
                  <p className="text-sm text-text-primary truncate">{o.display_name || o.value}</p>
                  <p className="text-xs text-text-secondary truncate">
                    {o.display_name ? `${o.value} · ` : ''}
                    {t('other_contacts_seen', { count: o.seen_count, module: o.source_module })}
                  </p>
                </div>
                <button type="button" onClick={() => save(o)} disabled={busy === o.id}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm text-primary hover:bg-primary-light disabled:opacity-50">
                  <UserPlus size={15} />{t('other_contacts_save')}
                </button>
                <button type="button" onClick={() => dismiss(o)} disabled={busy === o.id}
                  title={t('other_contacts_dismiss')} aria-label={t('other_contacts_dismiss')}
                  className="w-8 h-8 rounded-full flex items-center justify-center text-text-secondary
                             opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-surface-2 disabled:opacity-50">
                  <X size={16} />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
