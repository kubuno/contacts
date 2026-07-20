/**
 * Globally-mounted contact picker (slot `app-dialogs`).
 *
 * Rendered by the host shell in every route, so a CONSUMER module (chat…) can
 * open it purely through `ModuleServiceRegistry.call('contacts', 'pickContact')`
 * — no cross-module import, no route change. The pending promise lives in
 * `contactPickerStore` (drive's `filesDialogStore` pattern) and is settled here.
 */
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Loader2, Search, Users } from 'lucide-react'
import { Button, FloatingWindow, Input } from '@ui'
import ContactAvatar from './ContactAvatar'
import { contactsApi, type Contact } from './api'
import { contactEnvelope } from './ContactsDataCard'
import { useContactPickerStore, type ContactPickerOptions } from './contactPickerStore'
import type { KubunoDataEnvelope } from './kubunoData'

interface Props {
  opts: ContactPickerOptions
  onClose: (envelope: KubunoDataEnvelope | null) => void
}

function ContactPickerInner({ opts, onClose }: Props) {
  const { t } = useTranslation('contacts')
  const [query, setQuery] = useState('')
  const [contacts, setContacts] = useState<Contact[]>([])
  const [loading, setLoading] = useState(true)
  const debRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => {
    let cancelled = false
    clearTimeout(debRef.current)
    setLoading(true)
    debRef.current = setTimeout(() => {
      contactsApi.listContacts({ q: query || undefined, limit: 50 })
        .then(res => { if (!cancelled) setContacts(res.data.contacts ?? []) })
        .catch(() => { if (!cancelled) setContacts([]) })
        .finally(() => { if (!cancelled) setLoading(false) })
    }, query ? 250 : 0)
    return () => { cancelled = true; clearTimeout(debRef.current) }
  }, [query])

  return (
    <FloatingWindow
      title={opts.title ?? t('picker_title', { defaultValue: 'Choisir un contact' })}
      icon={<Users size={17} className="text-primary" />}
      onClose={() => onClose(null)}
      defaultWidth={520}
      defaultHeight={520}
      resizable
    >
      <div className="flex flex-col flex-1 min-h-0">
        {/* Search */}
        <div className="px-4 py-3 border-b border-border flex-shrink-0">
          <div className="relative">
            <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-tertiary pointer-events-none" />
            <Input
              autoFocus
              value={query}
              onChange={e => setQuery(e.target.value)}
              placeholder={t('contacts_search_ph', { defaultValue: 'Rechercher dans les contacts…' })}
              className="pl-8 w-full"
            />
          </div>
        </div>

        {/* Results */}
        <div className="flex-1 overflow-y-auto p-2">
          {loading ? (
            <div className="flex items-center justify-center h-full">
              <Loader2 size={20} className="animate-spin text-text-tertiary" />
            </div>
          ) : contacts.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-text-tertiary gap-2">
              <Users size={32} strokeWidth={1} />
              <p className="text-xs">{t('picker_empty', { defaultValue: 'Aucun contact' })}</p>
            </div>
          ) : (
            <div className="space-y-0.5">
              {contacts.map(c => (
                <button
                  key={c.id}
                  onClick={() => onClose(contactEnvelope(c))}
                  className="flex items-center gap-3 w-full px-3 py-2 rounded-lg hover:bg-surface-1 text-left"
                >
                  <ContactAvatar contact={c} size="sm" />
                  <span className="min-w-0 flex-1">
                    <span className="block text-sm text-text-primary truncate">
                      {c.display_name || t('no_name')}
                    </span>
                    {(c.emails[0]?.value || c.organization) && (
                      <span className="block text-xs text-text-secondary truncate">
                        {c.emails[0]?.value || c.organization}
                      </span>
                    )}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-5 py-3 border-t border-border bg-surface-1 flex-shrink-0">
          <Button variant="secondary" size="sm" onClick={() => onClose(null)}>
            {t('cancel', { defaultValue: 'Annuler' })}
          </Button>
        </div>
      </div>
    </FloatingWindow>
  )
}

export default function ContactPickerDialog() {
  const opts    = useContactPickerStore(s => s.pickerOpts)
  const resolve = useContactPickerStore(s => s._resolve)

  if (!opts) return null

  return <ContactPickerInner opts={opts} onClose={resolve} />
}
