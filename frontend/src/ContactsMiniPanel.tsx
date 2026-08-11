import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Copy, Mail, Phone, Search, Star } from 'lucide-react'
import { Input, Spinner } from '@ui'
import { contactsApi, type Contact } from './api'
import { CONTACTS_BASE } from './hashRoute'
import ContactAvatar from './ContactAvatar'

/**
 * Contacts side panel — the companion you open while writing somewhere else.
 *
 * Deliberately not a second address book: no groups, no labels, no editing. A
 * search box, the starred contacts as a default list, and the two actions worth
 * having in place (copy the address, start a call). Anything more and it competes
 * with the module itself rather than saving a round trip to it.
 */
export default function ContactsMiniPanel() {
  const { t } = useTranslation('contacts')
  const navigate = useNavigate()
  const [q, setQ] = useState('')
  const [copied, setCopied] = useState<string | null>(null)

  const query = q.trim()
  // Two distinct queries rather than one conditional: the starred list stays in
  // cache while the user types, so clearing the box is instant.
  const { data, isLoading } = useQuery({
    queryKey: ['contacts-mini', query],
    queryFn: () => (query
      ? contactsApi.listContacts({ q: query, limit: 20 })
      : contactsApi.listContacts({ starred: true, limit: 20 })
    ).then(r => r.data),
  })

  const contacts = data?.contacts ?? []

  const copy = async (value: string, id: string) => {
    try {
      await navigator.clipboard.writeText(value)
    } catch {
      // Insecure context (plain http on a LAN address) has no clipboard API.
      const el = document.createElement('textarea')
      el.value = value
      el.style.cssText = 'position:fixed;opacity:0'
      document.body.appendChild(el); el.select()
      try { document.execCommand('copy') } catch { /* ignored */ }
      document.body.removeChild(el)
    }
    setCopied(id)
    setTimeout(() => setCopied(c => (c === id ? null : c)), 1500)
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex-shrink-0 px-3 pt-3 pb-2">
        <Input
          value={q}
          onChange={e => setQ(e.target.value)}
          placeholder={t('search_placeholder', { defaultValue: 'Rechercher un contact…' })}
          leftIcon={<Search size={15} />}
          className="w-full pl-9"
        />
      </div>

      <div className="flex items-center gap-1.5 px-4 pb-1 text-text-tertiary" style={{ fontSize: 'var(--kb-text-meta)' }}>
        {!query && <Star size={12} />}
        <span className="uppercase tracking-wide">
          {query
            ? t('mini_results', { defaultValue: 'Résultats' })
            : t('mini_starred', { defaultValue: 'Favoris' })}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        {isLoading ? (
          <div className="flex justify-center py-6"><Spinner /></div>
        ) : contacts.length === 0 ? (
          <p className="px-2 py-4 text-text-tertiary" style={{ fontSize: 'var(--kb-text-meta)' }}>
            {query
              ? t('mini_no_results', { defaultValue: 'Aucun contact ne correspond.' })
              : t('mini_no_starred', { defaultValue: 'Aucun favori. Cherchez un contact ci-dessus.' })}
          </p>
        ) : (
          <ul className="space-y-0.5">
            {contacts.map(c => (
              <Row key={c.id} contact={c} copied={copied === c.id}
                   onOpen={() => navigate(`${CONTACTS_BASE}/${c.id}`)}
                   onCopy={v => copy(v, c.id)} />
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}

function Row({ contact, copied, onOpen, onCopy }: {
  contact: Contact
  copied:  boolean
  onOpen:  () => void
  onCopy:  (value: string) => void
}) {
  const email = contact.emails?.[0]?.value
  const phone = contact.phones?.[0]?.value
  const sub   = contact.job_title || contact.organization || email

  return (
    <li className="group flex items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-surface-1">
      <button type="button" onClick={onOpen} className="flex min-w-0 flex-1 items-center gap-2 text-left">
        <ContactAvatar contact={contact} size="sm" />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-text-primary" style={{ fontSize: 'var(--kb-text-body)' }}>
            {contact.display_name}
          </span>
          {sub && (
            <span className="block truncate text-text-tertiary" style={{ fontSize: 'var(--kb-text-meta)' }}>
              {copied ? '✓' : sub}
            </span>
          )}
        </span>
      </button>

      {/* Actions appear on hover but stay reachable by keyboard (focus-within). */}
      <span className="flex flex-shrink-0 items-center gap-0.5 opacity-0 transition-opacity
                       group-hover:opacity-100 group-focus-within:opacity-100">
        {email && (
          <IconBtn label="Copier l'adresse" onClick={() => onCopy(email)}><Copy size={14} /></IconBtn>
        )}
        {email && (
          <IconBtn label="Écrire" onClick={() => { window.location.href = `mailto:${email}` }}><Mail size={14} /></IconBtn>
        )}
        {phone && (
          <IconBtn label="Appeler" onClick={() => { window.location.href = `tel:${phone}` }}><Phone size={14} /></IconBtn>
        )}
      </span>
    </li>
  )
}

function IconBtn({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className="flex h-7 w-7 items-center justify-center rounded-md text-text-tertiary transition-colors
                 hover:bg-surface-2 hover:text-text-primary
                 focus:outline-none focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-primary"
    >
      {children}
    </button>
  )
}
