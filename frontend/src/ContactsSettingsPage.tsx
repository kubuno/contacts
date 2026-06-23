import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Users, ArrowLeft, ExternalLink, Check, Copy } from 'lucide-react'
import { Toggle, Button, Radio } from '@ui'
import { useModulePrefs } from './userPrefs'
import { contactsApi } from './api'
import { useCopy } from './widgets'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

interface ContactsPrefs {
  sort:        string   // 'first' | 'last'
  nameFormat:  string   // 'first_last' | 'last_first'
  defaultView: string   // 'list' | 'grid'
  showAvatars: boolean
  phoneFormat: string   // 'international' | 'national' | 'raw'
}

const DEFAULT_PREFS: ContactsPrefs = {
  sort: 'first', nameFormat: 'first_last', defaultView: 'list',
  showAvatars: true, phoneFormat: 'international',
}

// ── Mail-style layout helpers ───────────────────────────────────────────────────

function SettingsRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-start gap-8 py-4 border-b border-[#e8eaed] last:border-0">
      <div className="w-60 flex-shrink-0">
        <p className="text-sm text-[#202124] font-normal">{label}</p>
        {description && <p className="text-xs text-text-tertiary mt-0.5 leading-relaxed">{description}</p>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}

function RadioGroup({ options, value, onChange }: {
  options: { value: string; label: string }[]; value: string; onChange: (v: string) => void
}) {
  return (
    <div className="flex flex-col items-start gap-2">
      {options.map(opt => (
        <Radio key={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} label={opt.label} />
      ))}
    </div>
  )
}

// ── Préférences tab (per-user) ──────────────────────────────────────────────────

function PreferencesTab() {
  const { t } = useTranslation('contacts')
  const { prefs: saved, update } = useModulePrefs<ContactsPrefs>('contacts', DEFAULT_PREFS)
  const [prefs, setPrefs] = useState<ContactsPrefs>(saved)
  const [savedFlag, setSavedFlag] = useState(false)
  const [busy, setBusy] = useState(false)

  const set = <K extends keyof ContactsPrefs>(key: K, value: ContactsPrefs[K]) =>
    setPrefs(p => ({ ...p, [key]: value }))

  const save = async () => {
    setBusy(true)
    try {
      await update(prefs)
      setSavedFlag(true)
      setTimeout(() => setSavedFlag(false), 2500)
    } finally { setBusy(false) }
  }

  return (
    <div>
      <SettingsRow
        label={t('contacts_pref_sort', { defaultValue: 'Tri par défaut' })}
        description={t('contacts_pref_sort_desc', { defaultValue: 'Champ utilisé pour classer la liste des contacts.' })}
      >
        <RadioGroup
          value={prefs.sort}
          onChange={v => set('sort', v)}
          options={[
            { value: 'first', label: t('contacts_pref_sort_first', { defaultValue: 'Par prénom' }) },
            { value: 'last',  label: t('contacts_pref_sort_last',  { defaultValue: 'Par nom' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('contacts_pref_name_format', { defaultValue: 'Affichage du nom' })}
        description={t('contacts_pref_name_format_desc', { defaultValue: 'Ordre du prénom et du nom dans la liste.' })}
      >
        <RadioGroup
          value={prefs.nameFormat}
          onChange={v => set('nameFormat', v)}
          options={[
            { value: 'first_last', label: t('contacts_pref_name_first_last', { defaultValue: 'Prénom Nom' }) },
            { value: 'last_first', label: t('contacts_pref_name_last_first', { defaultValue: 'Nom Prénom' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('contacts_pref_view', { defaultValue: 'Vue par défaut' })}>
        <RadioGroup
          value={prefs.defaultView}
          onChange={v => set('defaultView', v)}
          options={[
            { value: 'list', label: t('contacts_pref_view_list', { defaultValue: 'Liste' }) },
            { value: 'grid', label: t('contacts_pref_view_grid', { defaultValue: 'Grille' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('contacts_pref_phone', { defaultValue: 'Format du téléphone' })}
        description={t('contacts_pref_phone_desc', { defaultValue: 'Format d\'affichage par défaut des numéros de téléphone.' })}
      >
        <RadioGroup
          value={prefs.phoneFormat}
          onChange={v => set('phoneFormat', v)}
          options={[
            { value: 'international', label: t('contacts_pref_phone_intl', { defaultValue: 'International (+33 6 12 34 56 78)' }) },
            { value: 'national',     label: t('contacts_pref_phone_national', { defaultValue: 'National (06 12 34 56 78)' }) },
            { value: 'raw',          label: t('contacts_pref_phone_raw', { defaultValue: 'Brut (tel que saisi)' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('contacts_pref_avatars', { defaultValue: 'Avatars' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.showAvatars} onChange={() => set('showAvatars', !prefs.showAvatars)} />
          <span className="text-sm text-text-primary">{t('contacts_pref_avatars_on', { defaultValue: 'Afficher les avatars dans la liste' })}</span>
        </label>
      </SettingsRow>

      <div className="pt-5 flex items-center gap-3">
        <Button onClick={save} loading={busy}>
          {savedFlag
            ? <><Check size={14} className="mr-1.5 inline" />{t('contacts_settings_saved', { defaultValue: 'Enregistré' })}</>
            : t('contacts_settings_save_changes', { defaultValue: 'Enregistrer les modifications' })}
        </Button>
        <Button variant="ghost" onClick={() => setPrefs(saved)}>
          {t('common_cancel', { defaultValue: 'Annuler' })}
        </Button>
      </div>
    </div>
  )
}

// ── CardDAV tab (per-user, reuses existing contacts CardDAV token API) ──────────

function DavRow({ label, value, onCopy }: { label: string; value: string; onCopy: (v: string) => void }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-text-secondary w-32 flex-shrink-0">{label}</span>
      <code className="flex-1 text-xs bg-surface-1 rounded px-2 py-1 truncate">{value}</code>
      <button onClick={() => onCopy(value)} className="p-1 rounded hover:bg-surface-2 text-text-secondary"><Copy size={13} /></button>
    </div>
  )
}

function CardDavTab() {
  const { t } = useTranslation('contacts')
  const [dav, setDav] = useState<{ configured: boolean; username: string; path: string } | null>(null)
  const [davCreds, setDavCreds] = useState<{ token: string; username: string; url: string } | null>(null)
  const [copied, copy] = useCopy()

  useEffect(() => {
    contactsApi.cardDavInfo().then(r => setDav(r.data))
  }, [])

  async function genDav() {
    const r = await contactsApi.cardDavGenerate()
    setDavCreds(r.data)
    setDav({ configured: true, username: r.data.username, path: '/dav' })
  }
  async function revokeDav() {
    await contactsApi.cardDavRevoke()
    setDav({ configured: false, username: '', path: '/dav' })
    setDavCreds(null)
  }

  const davUrl = davCreds ? `${window.location.origin}/api/v1/contacts${davCreds.url}` : ''

  return (
    <div>
      <h3 className="text-sm font-semibold text-text-primary mb-1">{t('set_carddav', { defaultValue: 'Sync (CardDAV)' })}</h3>
      <p className="text-xs text-text-secondary mb-4">{t('set_carddav_desc', { defaultValue: 'Synchronisez vos contacts avec votre téléphone ou votre ordinateur. Utilisez ces identifiants dans votre application CardDAV.' })}</p>
      {dav?.configured ? (
        <div className="space-y-2">
          {davCreds && (
            <>
              <p className="text-xs text-warning bg-warning-light rounded-lg px-3 py-2">{t('set_carddav_warn', { defaultValue: 'Copiez ces identifiants maintenant : le mot de passe ne sera plus affiché.' })}</p>
              <DavRow label={t('set_carddav_url', { defaultValue: 'URL' })} value={davUrl} onCopy={copy} />
              <DavRow label={t('set_carddav_user', { defaultValue: 'Utilisateur' })} value={davCreds.username} onCopy={copy} />
              <DavRow label={t('set_carddav_pwd', { defaultValue: 'Mot de passe' })} value={davCreds.token} onCopy={copy} />
            </>
          )}
          <div className="flex gap-2 pt-1">
            <button onClick={genDav} className="text-sm px-3 py-1.5 rounded-lg border border-border hover:bg-surface-1">{t('set_carddav_regen', { defaultValue: 'Régénérer' })}</button>
            <button onClick={revokeDav} className="text-sm px-3 py-1.5 rounded-lg text-danger hover:bg-danger-light">{t('set_carddav_revoke', { defaultValue: 'Révoquer' })}</button>
          </div>
        </div>
      ) : (
        <button onClick={genDav} className="text-sm px-3 py-1.5 rounded-lg bg-primary text-white hover:bg-primary-hover">{t('set_carddav_generate', { defaultValue: 'Générer des identifiants' })}</button>
      )}
      {copied && <span className="text-xs text-success ml-2">{t('copied', { defaultValue: 'Copié' })}</span>}
    </div>
  )
}

// ── About tab ───────────────────────────────────────────────────────────────────

function AboutTab() {
  const { t } = useTranslation('contacts')
  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
        <div className="w-10 h-10 rounded-xl bg-blue-100 flex items-center justify-center shrink-0">
          <Users size={20} className="text-blue-600" />
        </div>
        <div>
          <p className="text-sm font-semibold text-text-primary">Kubuno Contacts</p>
          <p className="text-xs text-text-tertiary">v0.1.0 · {t('contacts_official_module', { defaultValue: 'Module officiel' })}</p>
        </div>
        <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">Rust</span>
      </div>
      <div className="px-5 py-4">
        <a href="https://github.com/kubuno/kubuno" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
          <ExternalLink size={13} /> github.com/kubuno/kubuno
        </a>
      </div>
    </div>
  )
}

// ── Main page (mail-style breadcrumb + tab bar) ─────────────────────────────────

type Tab = 'preferences' | 'carddav' | 'about'

export default function ContactsSettingsPage() {
  const { t } = useTranslation('contacts')
  const [tab, setTab] = useState<Tab>('preferences')

  const tabs: { id: Tab; label: string }[] = [
    { id: 'preferences', label: t('contacts_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'carddav',     label: t('contacts_tab_carddav', { defaultValue: 'CardDAV' }) },
    { id: 'about',       label: t('contacts_tab_about', { defaultValue: 'À propos' }) },
  ]

  return (
    <div className="flex flex-col h-full bg-white overflow-hidden">
      {/* Breadcrumb header */}
      <div className="flex items-center gap-2 px-6 py-2.5 border-b border-[#e8eaed] flex-shrink-0" style={{ background: '#f8f9fa' }}>
        <Link to="/contacts" className="flex items-center gap-1.5 text-sm text-[#1a73e8] hover:underline">
          <ArrowLeft size={14} />
          Contacts
        </Link>
        <span className="text-text-tertiary text-sm">/</span>
        <div className="flex items-center gap-1.5">
          <Users size={15} className="text-text-secondary" />
          <span className="text-sm text-text-primary">{t('contacts_settings_title', { defaultValue: 'Réglages' })}</span>
        </div>
      </div>

      {/* Tab bar (Gmail-style) */}
      <div className="flex items-end border-b border-[#e8eaed] px-4 flex-shrink-0 overflow-x-auto" style={{ background: '#fff' }}>
        {tabs.map(tb => (
          <button key={tb.id} onClick={() => setTab(tb.id)}
            className={`px-4 py-3 text-sm border-b-2 -mb-px transition-colors whitespace-nowrap ${
              tab === tb.id ? 'border-[#1a73e8] text-[#1a73e8] font-medium' : 'border-transparent text-[#5f6368] hover:text-[#202124] hover:bg-[#f1f3f4]'}`}>
            {tb.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-6">
          {tab === 'preferences' && <PreferencesTab />}
          {tab === 'carddav'     && <CardDavTab />}
          {tab === 'about'       && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
