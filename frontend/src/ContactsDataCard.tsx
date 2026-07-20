/**
 * `contacts.contact` envelopes: card renderer + envelope builder for contacts
 * copied from the context menu ("Copier pour Kubuno") or picked through the
 * `pickContact` service. Registered on `core.data-card` from `entry.ts`;
 * consumers (chat…) resolve it dynamically and never import contacts' code.
 */
import { useNavigate } from 'react-router-dom'
import { Mail, Phone, ExternalLink, User } from 'lucide-react'
import type { Contact } from './api'
import type { DataCardProps, KubunoDataEnvelope } from './kubunoData'

export interface ContactCardData {
  id: string
  display_name: string
  organization?: string
  job_title?: string
  emails: { value: string; type?: string }[]
  phones: { value: string; type?: string }[]
  has_avatar: boolean
  avatar_color?: string
}

/** Deep link to the contact's detail panel (handled by `ContactsApp`). */
export function contactHref(id: string): string {
  return `/contacts?contact=${encodeURIComponent(id)}`
}

export function contactEnvelope(c: Contact): KubunoDataEnvelope {
  const href = contactHref(c.id)
  const email = c.emails?.[0]?.value
  const phone = c.phones?.[0]?.value
  const data: ContactCardData = {
    id: c.id,
    display_name: c.display_name,
    organization: c.organization || undefined,
    job_title: c.job_title || undefined,
    emails: (c.emails ?? []).map(f => ({ value: f.value, type: f.type })),
    phones: (c.phones ?? []).map(f => ({ value: f.value, type: f.type })),
    has_avatar: !!c.avatar_path,
    avatar_color: c.avatar_color || undefined,
  }
  return {
    kubuno: 1,
    type: 'contacts.contact',
    module: 'contacts',
    title: c.display_name,
    text: [c.display_name, email, phone].filter(Boolean).join('\n'),
    href,
    data,
  }
}

/** Initials of a display name ("Ada Lovelace" → "AL"). */
function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length === 0) return ''
  if (parts.length === 1) return parts[0][0].toUpperCase()
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
}

export default function ContactsDataCard({ envelope }: DataCardProps) {
  const navigate = useNavigate()
  const d = envelope.data as ContactCardData | null
  if (!d || typeof d.display_name !== 'string') return null
  // Initials only: no remote <img>, so the card stays robust inside consumer
  // modules (avatar route may 401/404 for another user or a deleted contact).
  const initials = initialsOf(d.display_name)
  const email = d.emails?.[0]?.value
  const phone = d.phones?.[0]?.value
  const subtitle = [d.job_title, d.organization].filter(Boolean).join(' · ')

  return (
    <div
      className="w-72 max-w-full rounded-xl border border-border bg-surface-0 overflow-hidden cursor-pointer hover:border-strong transition-colors"
      onClick={() => { if (envelope.href) navigate(envelope.href) }}
      role="button"
    >
      <div className="px-3 py-2.5 flex items-center gap-2.5">
        <span
          className="w-9 h-9 rounded-full flex items-center justify-center text-white text-sm font-semibold flex-shrink-0"
          style={{ backgroundColor: d.avatar_color || '#1a73e8' }}
        >{initials || <User size={16} />}</span>
        <div className="min-w-0 flex-1">
          <p className="text-xs font-semibold text-text-primary truncate">{d.display_name}</p>
          {subtitle && <p className="text-[11px] text-text-tertiary truncate">{subtitle}</p>}
          <div className="flex flex-col gap-0.5 mt-0.5">
            {email && <span className="text-[11px] text-text-secondary flex items-center gap-1 truncate"><Mail size={11} className="flex-shrink-0" /> {email}</span>}
            {phone && <span className="text-[11px] text-text-secondary flex items-center gap-1 truncate"><Phone size={11} className="flex-shrink-0" /> {phone}</span>}
          </div>
        </div>
        <ExternalLink size={13} className="text-text-tertiary flex-shrink-0" />
      </div>
    </div>
  )
}
