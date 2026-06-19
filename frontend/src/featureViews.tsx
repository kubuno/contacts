import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Cake, Bell, Trash2, Check, GitMerge, X, Copy, Download, BarChart3,
} from 'lucide-react'
import {
  contactsApi, type UpcomingDate, type Reminder, type DuplicateGroup, type ContactStats,
} from './api'
import { useContactsStore } from './store'
import { useCopy } from './widgets'

// ── Tiny avatar (initials) ──────────────────────────────────────────────────
function MiniAvatar({ name, color, size = 40 }: { name: string; color: string; size?: number }) {
  const initials = name.split(/\s+/).slice(0, 2).map(s => s[0]?.toUpperCase() ?? '').join('')
  return (
    <span className="flex items-center justify-center rounded-full text-white font-medium flex-shrink-0"
      style={{ width: size, height: size, backgroundColor: color, fontSize: size * 0.4 }}>
      {initials || '?'}
    </span>
  )
}

function ViewHeader({ icon, title, count }: { icon: React.ReactNode; title: string; count?: number }) {
  return (
    <div className="px-6 pt-4 pb-3 flex items-center gap-3">
      <span className="text-primary">{icon}</span>
      <h1 className="text-2xl font-normal text-text-primary">
        {title}{count !== undefined && count > 0 && <span className="text-text-secondary ml-2 text-xl">({count})</span>}
      </h1>
    </div>
  )
}

// ── Birthdays ───────────────────────────────────────────────────────────────
export function BirthdaysView() {
  const { t } = useTranslation('contacts')
  const [dates, setDates] = useState<UpcomingDate[]>([])
  const setSelectedId = useContactsStore(s => s.setSelectedId)

  useEffect(() => { contactsApi.birthdays(365).then(r => setDates(r.data.dates)) }, [])

  function when(d: UpcomingDate) {
    if (d.days_until === 0) return t('bday_today')
    if (d.days_until === 1) return t('bday_tomorrow')
    return t('bday_in_days', { count: d.days_until })
  }
  async function remind(d: UpcomingDate) {
    await contactsApi.createReminder({
      contact_id: d.contact_id, kind: 'birthday',
      message: `${d.label} — ${d.display_name}`,
      remind_at: new Date(d.next_occurrence + 'T09:00:00').toISOString(),
      recurrence: 'yearly',
    })
    useContactsStore.getState().fetchDueCount()
  }

  return (
    <div className="flex-1 overflow-y-auto">
      <ViewHeader icon={<Cake size={24} />} title={t('bday_title')} count={dates.length} />
      {!dates.length ? (
        <p className="px-6 text-text-secondary">{t('bday_empty')}</p>
      ) : (
        <div className="px-4 space-y-1">
          {dates.map((d, i) => (
            <div key={i} className="flex items-center gap-3 px-3 py-2 rounded-xl hover:bg-surface-1">
              <MiniAvatar name={d.display_name} color={d.avatar_color} />
              <button onClick={() => setSelectedId(d.contact_id)} className="flex-1 text-left min-w-0">
                <p className="text-sm font-medium text-text-primary truncate">{d.display_name}</p>
                <p className="text-xs text-text-secondary">
                  {d.label}{d.age != null && <> · {t('bday_turns', { age: d.age })}</>}
                </p>
              </button>
              <span className={`text-xs font-medium ${d.days_until <= 1 ? 'text-primary' : 'text-text-secondary'}`}>{when(d)}</span>
              <button onClick={() => remind(d)} title={t('bday_remind')}
                className="p-1.5 rounded-full hover:bg-surface-2 text-text-secondary"><Bell size={15} /></button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// ── Reminders ───────────────────────────────────────────────────────────────
export function RemindersView() {
  const { t } = useTranslation('contacts')
  const [reminders, setReminders] = useState<Reminder[]>([])
  const setSelectedId = useContactsStore(s => s.setSelectedId)

  function load() { contactsApi.listReminders(true).then(r => setReminders(r.data.reminders)) }
  useEffect(() => { load() }, [])

  async function done(r: Reminder) { await contactsApi.updateReminder(r.id, { is_done: !r.is_done }); load(); useContactsStore.getState().fetchDueCount() }
  async function del(r: Reminder) { await contactsApi.deleteReminder(r.id); load(); useContactsStore.getState().fetchDueCount() }

  return (
    <div className="flex-1 overflow-y-auto">
      <ViewHeader icon={<Bell size={24} />} title={t('rem_title')} count={reminders.filter(r => !r.is_done).length} />
      {!reminders.length ? (
        <p className="px-6 text-text-secondary">{t('rem_empty')}</p>
      ) : (
        <div className="px-4 space-y-1">
          {reminders.map(r => {
            const overdue = !r.is_done && new Date(r.remind_at) < new Date()
            return (
              <div key={r.id} className={`flex items-center gap-3 px-3 py-2 rounded-xl hover:bg-surface-1 ${r.is_done ? 'opacity-50' : ''}`}>
                <button onClick={() => done(r)}
                  className={`w-5 h-5 rounded-full border flex items-center justify-center ${r.is_done ? 'bg-success border-success' : 'border-border hover:border-primary'}`}>
                  {r.is_done && <Check size={12} className="text-white" />}
                </button>
                <MiniAvatar name={r.contact_name} color={r.contact_avatar_color} size={32} />
                <button onClick={() => setSelectedId(r.contact_id)} className="flex-1 text-left min-w-0">
                  <p className={`text-sm text-text-primary truncate ${r.is_done ? 'line-through' : ''}`}>{r.message || r.contact_name}</p>
                  <p className="text-xs text-text-secondary">{r.contact_name} · {new Date(r.remind_at).toLocaleString()}</p>
                </button>
                {overdue && <span className="text-xs font-medium text-danger">{t('rem_overdue')}</span>}
                <button onClick={() => del(r)} className="p-1.5 rounded-full hover:bg-surface-2 text-text-secondary"><Trash2 size={15} /></button>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

// ── Duplicates ──────────────────────────────────────────────────────────────
export function DuplicatesView() {
  const { t } = useTranslation('contacts')
  const [groups, setGroups] = useState<DuplicateGroup[]>([])
  const fetchContacts = useContactsStore(s => s.fetchContacts)

  function load() { contactsApi.findDuplicates().then(r => setGroups(r.data.groups)) }
  useEffect(() => { load() }, [])

  async function merge(g: DuplicateGroup, primaryIdx: number) {
    const primary = g.contacts[primaryIdx]
    const dups = g.contacts.filter((_, i) => i !== primaryIdx).map(c => c.id)
    await contactsApi.mergeContacts(primary.id, dups)
    load(); fetchContacts()
  }
  async function ignore(g: DuplicateGroup) {
    if (g.contacts.length >= 2) await contactsApi.ignoreDuplicate(g.contacts[0].id, g.contacts[1].id)
    load()
  }

  return (
    <div className="flex-1 overflow-y-auto">
      <ViewHeader icon={<GitMerge size={24} />} title={t('dup_title')} count={groups.length} />
      {!groups.length ? (
        <p className="px-6 text-text-secondary">{t('dup_empty')}</p>
      ) : (
        <div className="px-4 space-y-3 pb-6">
          {groups.map((g, gi) => (
            <div key={gi} className="border border-border rounded-xl p-3">
              <p className="text-xs text-text-secondary mb-2">{t('dup_reason')} : {g.reason}</p>
              <div className="space-y-2">
                {g.contacts.map((c, ci) => (
                  <div key={c.id} className="flex items-center gap-3">
                    <MiniAvatar name={c.display_name} color={c.avatar_color} size={32} />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm text-text-primary truncate">{c.display_name}</p>
                      <p className="text-xs text-text-secondary truncate">
                        {[c.emails[0]?.value, c.phones[0]?.value, c.organization].filter(Boolean).join(' · ')}
                      </p>
                    </div>
                    <button onClick={() => merge(g, ci)}
                      className="text-xs px-2.5 py-1 rounded-full bg-primary text-white hover:bg-primary-hover">
                      {t('dup_keep_primary')}
                    </button>
                  </div>
                ))}
              </div>
              <div className="flex items-center gap-2 mt-3 pt-2 border-t border-border">
                <button onClick={() => ignore(g)}
                  className="flex items-center gap-1 text-xs text-text-secondary hover:text-text-primary">
                  <X size={13} /> {t('dup_ignore')}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// ── Settings ────────────────────────────────────────────────────────────────
export function SettingsView() {
  const { t } = useTranslation('contacts')
  const [prefs, setPrefs] = useState<Record<string, unknown>>({})
  const [stats, setStats] = useState<ContactStats | null>(null)
  const [dav, setDav] = useState<{ configured: boolean; username: string; path: string } | null>(null)
  const [davCreds, setDavCreds] = useState<{ token: string; username: string; url: string } | null>(null)
  const [copied, copy] = useCopy()

  useEffect(() => {
    contactsApi.getSettings().then(r => setPrefs(r.data.settings))
    contactsApi.getStats().then(r => setStats(r.data.stats))
    contactsApi.cardDavInfo().then(r => setDav(r.data))
  }, [])

  function savePref(key: string, value: unknown) {
    const next = { ...prefs, [key]: value }
    setPrefs(next)
    contactsApi.updateSettings({ [key]: value })
  }
  async function genDav() { const r = await contactsApi.cardDavGenerate(); setDavCreds(r.data); setDav({ configured: true, username: r.data.username, path: '/dav' }) }
  async function revokeDav() { await contactsApi.cardDavRevoke(); setDav({ configured: false, username: '', path: '/dav' }); setDavCreds(null) }

  const davUrl = davCreds ? `${window.location.origin}/api/v1/contacts${davCreds.url}` : ''

  return (
    <div className="flex-1 overflow-y-auto">
      <ViewHeader icon={<BarChart3 size={24} />} title={t('set_title')} />
      <div className="px-6 max-w-2xl space-y-8 pb-10">

        {/* Display */}
        <section>
          <h3 className="text-sm font-semibold text-text-primary mb-3">{t('set_display')}</h3>
          <label className="flex items-center justify-between py-2">
            <span className="text-sm text-text-secondary">{t('set_default_view')}</span>
            <select value={(prefs.default_view as string) ?? 'list'} onChange={e => savePref('default_view', e.target.value)}
              className="text-sm border border-border rounded-lg px-2 py-1 bg-white">
              <option value="list">{t('view_list')}</option>
              <option value="grid">{t('view_grid')}</option>
              <option value="table">{t('view_table')}</option>
            </select>
          </label>
          <label className="flex items-center justify-between py-2">
            <span className="text-sm text-text-secondary">{t('set_name_format')}</span>
            <select value={(prefs.name_format as string) ?? 'first_last'} onChange={e => savePref('name_format', e.target.value)}
              className="text-sm border border-border rounded-lg px-2 py-1 bg-white">
              <option value="first_last">{t('set_name_first_last')}</option>
              <option value="last_first">{t('set_name_last_first')}</option>
            </select>
          </label>
        </section>

        {/* CardDAV sync */}
        <section>
          <h3 className="text-sm font-semibold text-text-primary mb-1">{t('set_carddav')}</h3>
          <p className="text-xs text-text-secondary mb-3">{t('set_carddav_desc')}</p>
          {dav?.configured ? (
            <div className="space-y-2">
              {davCreds && (
                <>
                  <p className="text-xs text-warning bg-warning-light rounded-lg px-3 py-2">{t('set_carddav_warn')}</p>
                  <DavRow label={t('set_carddav_url')} value={davUrl} onCopy={copy} />
                  <DavRow label={t('set_carddav_user')} value={davCreds.username} onCopy={copy} />
                  <DavRow label={t('set_carddav_pwd')} value={davCreds.token} onCopy={copy} />
                </>
              )}
              <div className="flex gap-2">
                <button onClick={genDav} className="text-sm px-3 py-1.5 rounded-lg border border-border hover:bg-surface-1">{t('set_carddav_regen')}</button>
                <button onClick={revokeDav} className="text-sm px-3 py-1.5 rounded-lg text-danger hover:bg-danger-light">{t('set_carddav_revoke')}</button>
              </div>
            </div>
          ) : (
            <button onClick={genDav} className="text-sm px-3 py-1.5 rounded-lg bg-primary text-white hover:bg-primary-hover">{t('set_carddav_generate')}</button>
          )}
          {copied && <span className="text-xs text-success ml-2">{t('copied')}</span>}
        </section>

        {/* Export */}
        <section>
          <h3 className="text-sm font-semibold text-text-primary mb-3">{t('set_export')}</h3>
          <div className="flex gap-2">
            <button onClick={() => contactsApi.exportVcf({})} className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-lg border border-border hover:bg-surface-1"><Download size={15} /> {t('set_export_vcf')}</button>
            <button onClick={() => contactsApi.exportCsv({})} className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-lg border border-border hover:bg-surface-1"><Download size={15} /> {t('set_export_csv')}</button>
          </div>
        </section>

        {/* Stats */}
        {stats && (
          <section>
            <h3 className="text-sm font-semibold text-text-primary mb-3">{t('set_stats')}</h3>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              <StatCard label={t('stat_total')} value={stats.total} />
              <StatCard label={t('stat_starred')} value={stats.starred} />
              <StatCard label={t('stat_groups')} value={stats.groups} />
              <StatCard label={t('stat_labels')} value={stats.labels} />
              <StatCard label={t('stat_with_email')} value={stats.with_email} />
              <StatCard label={t('stat_with_phone')} value={stats.with_phone} />
              <StatCard label={t('stat_completeness')} value={`${stats.completeness_pct}%`} />
            </div>
          </section>
        )}
      </div>
    </div>
  )
}

function DavRow({ label, value, onCopy }: { label: string; value: string; onCopy: (v: string) => void }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-text-secondary w-32 flex-shrink-0">{label}</span>
      <code className="flex-1 text-xs bg-surface-1 rounded px-2 py-1 truncate">{value}</code>
      <button onClick={() => onCopy(value)} className="p-1 rounded hover:bg-surface-2 text-text-secondary"><Copy size={13} /></button>
    </div>
  )
}

function StatCard({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-xl border border-border p-3">
      <p className="text-2xl font-semibold text-text-primary">{value}</p>
      <p className="text-xs text-text-secondary">{label}</p>
    </div>
  )
}
