import { Contact } from './api'
import { contactsApi } from './api'
import { User } from 'lucide-react'

interface Props {
  contact: Contact
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl'
  className?: string
}

const SIZES = {
  xs: 'w-6 h-6 text-xs',
  sm: 'w-8 h-8 text-sm',
  md: 'w-10 h-10 text-sm',
  lg: 'w-14 h-14 text-base',
  xl: 'w-20 h-20 text-xl',
  // Editor hero: the same weight the photo has in Google Contacts' form.
  '2xl': 'w-40 h-40 text-5xl',
}

export default function ContactAvatar({ contact, size = 'md', className = '' }: Props) {
  const cls = `${SIZES[size]} rounded-full flex items-center justify-center font-semibold text-white flex-shrink-0 overflow-hidden ${className}`

  if (contact.avatar_path) {
    return (
      <div className={cls}>
        {/* A data:/blob: path is a LOCAL preview (a photo just picked in the
            editor, before any upload): it is already the image. Rebuilding a
            server URL from the contact id would 404 — and on a contact that does
            not exist yet, there is no id at all. */}
        <img
          src={/^(data:|blob:)/.test(contact.avatar_path) ? contact.avatar_path : contactsApi.avatarUrl(contact.id)}
          alt={contact.display_name}
          className="w-full h-full object-cover"
          onError={e => { (e.target as HTMLImageElement).style.display = 'none' }}
        />
      </div>
    )
  }

  const initials = getInitials(contact)
  return (
    <div className={cls} style={{ backgroundColor: contact.avatar_color || '#1a73e8' }}>
      {initials || <User size={size === 'xs' ? 12 : size === 'sm' ? 14 : 18} />}
    </div>
  )
}

function getInitials(contact: Contact): string {
  if (contact.given_name && contact.family_name) {
    return (contact.given_name[0] + contact.family_name[0]).toUpperCase()
  }
  if (contact.display_name) {
    const parts = contact.display_name.trim().split(/\s+/)
    if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
    return contact.display_name[0].toUpperCase()
  }
  return ''
}
