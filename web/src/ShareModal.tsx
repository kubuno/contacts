import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X, Copy, Link2, Trash2 } from 'lucide-react'
import { contactsApi, type ContactsInstanceConfig, type Share } from './api'
import { useCopy } from './widgets'

export default function ShareModal({ contactId, onClose }: { contactId: string; onClose: () => void }) {
  const { t } = useTranslation('contacts')
  const [shares, setShares] = useState<Share[]>([])
  const [expires, setExpires] = useState<string>('7')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [copied, copy] = useCopy()

  // Instance policy: public links can be switched off, capped in duration, or
  // required to carry a password. Reflected here so the form never offers what
  // the server refuses — the server still refuses it either way.
  const [cfg, setCfg] = useState<ContactsInstanceConfig | null>(null)
  useEffect(() => {
    let alive = true
    contactsApi.getInstanceConfig()
      .then(r => { if (alive) setCfg(r.data) })
      .catch(() => {})
    return () => { alive = false }
  }, [])

  const maxDays = cfg?.share_max_expiry_days ?? 0
  const options = [7, 30].filter(d => maxDays <= 0 || d <= maxDays)

  function load() {
    contactsApi.listShares().then(r => setShares(r.data.shares.filter(s => s.contact_id === contactId)))
  }
  useEffect(() => { load() }, [contactId])

  async function create() {
    if (cfg?.share_password_required && !password.trim()) {
      setError(t('share_password_required'))
      return
    }
    const days = expires === 'never' ? undefined : Number(expires)
    try {
      await contactsApi.createShare({
        contact_id:      contactId,
        expires_in_days: days,
        password:        password.trim() || undefined,
      })
      setPassword('')
      setError(null)
      load()
    } catch {
      setError(t('share_disabled'))
    }
  }
  async function revoke(id: string) { await contactsApi.revokeShare(id); load() }

  const linkFor = (s: Share) => `${window.location.origin}/api/v1/contacts/shared/${s.token}`

  return (
    <div className="fixed inset-0 z-[1000] flex items-center justify-center bg-black/30" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-xl w-[440px] max-w-[90vw] p-5" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-medium text-text-primary flex items-center gap-2"><Link2 size={18} />{t('share_title')}</h2>
          <button onClick={onClose} className="p-1.5 rounded-full hover:bg-surface-2 text-text-secondary"><X size={16} /></button>
        </div>

        {cfg && !cfg.public_shares_enabled ? (
          <p className="text-sm text-text-secondary bg-surface-1 border border-border rounded-lg px-3 py-2 mb-4">
            {t('share_disabled')}
          </p>
        ) : (
          <>
            <div className="flex items-end gap-2 mb-2">
              <label className="flex-1">
                <span className="text-xs text-text-secondary">{t('share_expires')}</span>
                <select value={expires} onChange={e => setExpires(e.target.value)}
                  className="mt-1 w-full text-sm border border-border rounded-lg px-2 py-1.5 bg-white">
                  {options.includes(7)  && <option value="7">{t('share_7d')}</option>}
                  {options.includes(30) && <option value="30">{t('share_30d')}</option>}
                  {maxDays > 0
                    ? <option value={String(maxDays)}>{maxDays} j</option>
                    : <option value="never">{t('share_never')}</option>}
                </select>
              </label>
              <button onClick={create} className="px-3 py-2 rounded-lg bg-primary text-white text-sm hover:bg-primary-hover">{t('share_create')}</button>
            </div>
            {cfg?.share_password_required && (
              <label className="block mb-4">
                <span className="text-xs text-text-secondary">{t('share_password')}</span>
                <input type="password" value={password} onChange={e => setPassword(e.target.value)}
                  className="mt-1 w-full text-sm border border-border rounded-lg px-2 py-1.5 bg-white" />
              </label>
            )}
            {error && <p className="text-xs text-danger mb-3">{error}</p>}
          </>
        )}

        <p className="text-xs font-semibold text-text-secondary mb-2">{t('share_active')}</p>
        {!shares.length ? (
          <p className="text-sm text-text-secondary">{t('share_none')}</p>
        ) : (
          <div className="space-y-2">
            {shares.map(s => (
              <div key={s.id} className="flex items-center gap-2">
                <code className="flex-1 text-xs bg-surface-1 rounded px-2 py-1.5 truncate">{linkFor(s)}</code>
                <button onClick={() => copy(linkFor(s))} title={t('share_copy')} className="p-1.5 rounded hover:bg-surface-2 text-text-secondary"><Copy size={14} /></button>
                <button onClick={() => revoke(s.id)} title={t('share_revoke')} className="p-1.5 rounded hover:bg-danger-light text-text-secondary hover:text-danger"><Trash2 size={14} /></button>
              </div>
            ))}
          </div>
        )}
        {copied && <p className="text-xs text-success mt-2">{t('copied')}</p>}
      </div>
    </div>
  )
}
