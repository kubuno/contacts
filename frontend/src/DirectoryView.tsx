import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, UserPlus } from 'lucide-react'
import { DirectoryProfile, contactsApi } from './api'
import { useContactsStore } from './store'
import { Button, Input } from '@ui'

export default function DirectoryView() {
  const { t } = useTranslation('contacts')
  const [query, setQuery] = useState('')
  const [profiles, setProfiles] = useState<DirectoryProfile[]>([])
  const [loading, setLoading] = useState(false)
  const [added, setAdded] = useState<string | null>(null)
  const { fetchContacts } = useContactsStore()

  async function search(q: string) {
    setQuery(q)
    if (q.length < 2) { setProfiles([]); return }
    setLoading(true)
    try {
      const res = await contactsApi.searchDirectory(q)
      setProfiles(res.data.profiles)
    } finally {
      setLoading(false)
    }
  }

  async function addToContacts(profile: DirectoryProfile) {
    await contactsApi.addFromDirectory(profile.kubuno_user_id)
    await fetchContacts()
    setAdded(profile.display_name)
    setTimeout(() => setAdded(null), 3000)
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-xl mx-auto">
        <h2 className="text-lg font-semibold text-text-primary mb-4">{t('directory')}</h2>
        <div className="mb-6">
          <Input
            type="text"
            value={query}
            onChange={e => search(e.target.value)}
            placeholder={t('search_user')}
            leftIcon={<Search size={16} />}
          />
        </div>

        {loading && <p className="text-sm text-text-secondary text-center">{t('searching')}</p>}
        {added && (
          <p className="text-sm text-success bg-success/10 border border-success/20 rounded-lg px-3 py-2 mb-4">
            {t('contacts_added_to_contacts', { name: added })}
          </p>
        )}

        <div className="space-y-2">
          {profiles.map(p => (
            <div key={p.kubuno_user_id} className="flex items-center gap-3 p-3 bg-white border border-border rounded-xl">
              <div className="w-10 h-10 rounded-full bg-primary flex items-center justify-center text-white font-medium text-sm flex-shrink-0">
                {p.avatar_url
                  ? <img src={p.avatar_url} className="w-full h-full rounded-full object-cover" />
                  : p.display_name[0]?.toUpperCase()}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-text-primary">{p.display_name}</p>
                <p className="text-xs text-text-secondary">{p.email}</p>
                {p.job_title && <p className="text-xs text-text-secondary">{p.job_title}{p.department ? ` — ${p.department}` : ''}</p>}
              </div>
              <Button
                variant="secondary"
                size="sm"
                icon={<UserPlus size={12} />}
                onClick={() => addToContacts(p)}
              >
                {t('contacts_add')}
              </Button>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
