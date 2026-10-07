import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Building, UserPlus, Check } from 'lucide-react'
import { Spinner } from '@ui'
import { contactsApi, type DirectoryProfile } from '../api'
import { useContactsStore } from '../store'

/**
 * Everyone in the signed-in user's own organisational unit (and its sub-units),
 * read from the CORE's governed directory — this module never keeps a copy of
 * the account list, so the instance's sharing policy always decides what is
 * visible. An instance with the directory closed simply shows nobody here.
 *
 * These are accounts, not contacts: they are listed as they are, and can be
 * pulled into the address book one by one.
 */
export default function UnitMembersView() {
  const { t } = useTranslation('contacts')
  const { fetchContacts } = useContactsStore()
  const [members, setMembers] = useState<DirectoryProfile[] | null>(null)
  const [added, setAdded] = useState<Set<string>>(new Set())
  const [busy, setBusy] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    contactsApi.listUnitMembers()
      .then(list => { if (alive) setMembers(list) })
      .catch(() => { if (alive) setMembers([]) })
    return () => { alive = false }
  }, [])

  async function add(p: DirectoryProfile) {
    setBusy(p.kubuno_user_id)
    try {
      await contactsApi.addFromDirectory(p.kubuno_user_id)
      setAdded(prev => new Set(prev).add(p.kubuno_user_id))
      fetchContacts()
    } finally { setBusy(null) }
  }

  if (!members) return <div className="flex-1 flex justify-center py-16"><Spinner /></div>

  return (
    <div className="flex-1 overflow-y-auto px-6 py-6">
      <div className="max-w-2xl">
        <h1 className="text-2xl font-normal text-text-primary mb-1">{t('unit_members')}</h1>
        <p className="text-sm text-text-secondary mb-6">{t('unit_members_help')}</p>

        {!members.length ? (
          <div className="flex flex-col items-center gap-3 py-16 text-center">
            <Building size={40} className="text-text-tertiary" />
            <p className="text-sm text-text-secondary">{t('unit_members_empty')}</p>
          </div>
        ) : (
          <ul className="divide-y divide-border">
            {members.map(p => (
              <li key={p.kubuno_user_id} className="flex items-center gap-3 py-3">
                {p.avatar_url
                  ? <img src={p.avatar_url} alt="" className="w-9 h-9 rounded-full object-cover flex-shrink-0" />
                  : <span className="w-9 h-9 rounded-full bg-surface-2 flex items-center justify-center text-sm text-text-secondary flex-shrink-0">
                      {(p.display_name || '?').charAt(0).toUpperCase()}
                    </span>}
                <div className="flex-1 min-w-0">
                  <p className="text-sm text-text-primary truncate">{p.display_name}</p>
                  {p.email && <p className="text-xs text-text-secondary truncate">{p.email}</p>}
                </div>
                {added.has(p.kubuno_user_id) ? (
                  <span className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-success">
                    <Check size={15} />{t('contacts_added')}
                  </span>
                ) : (
                  <button type="button" onClick={() => add(p)} disabled={busy === p.kubuno_user_id}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm text-primary hover:bg-primary-light disabled:opacity-50">
                    <UserPlus size={15} />{t('contacts_add')}
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
