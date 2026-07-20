import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link as RouterLink, useLocation } from 'react-router-dom'
import {
  Users, Star, GitMerge, Download, Trash2, Plus, Trash, Cake, Bell, Clock,
  Archive, Settings, Tag, RotateCcw,
} from 'lucide-react'
import { useContactsStore, type View } from './store'
import { contactsApi } from './api'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog, Input } from '@ui'
import { SidebarNavItem } from '@kubuno/sdk'
import { hashTo, fromHash } from './hashRoute'

const LABEL_COLORS = ['#1a73e8', '#e8710a', '#1e8e3e', '#d93025', '#9334e6', '#12b5cb', '#f9ab00', '#e52592']

// Views reachable through a sidebar hash link. Anything else in the hash is
// none of our business and is left alone.
const HASH_VIEWS: readonly string[] = [
  'all', 'starred', 'birthdays', 'reminders', 'frequent', 'followup',
  'duplicates', 'archived', 'trashed', 'settings', 'group', 'label',
]

export default function ContactsSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('contacts')
  const {
    view, groups, labels, total, dueCount, activeGroupId, activeLabelId,
    setView, fetchGroups, fetchLabels, fetchDueCount, setEditorOpen, setImportOpen,
  } = useContactsStore()
  const [newLabel, setNewLabel] = useState('')
  const [showNewLabel, setShowNewLabel] = useState(false)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { hash } = useLocation()

  useEffect(() => { fetchLabels(); fetchDueCount() }, [])

  // The hash is the source of truth for the selected view: a direct link, a
  // sidebar click and the Back button all end up here.
  useEffect(() => {
    const r = fromHash(hash)
    if (!r || !HASH_VIEWS.includes(r.kind)) return
    setView(r.kind as View, r.id ?? undefined)
  }, [hash, setView])

  async function createLabel() {
    if (!newLabel.trim()) return
    const color = LABEL_COLORS[labels.length % LABEL_COLORS.length]
    await contactsApi.createLabel(newLabel.trim(), color)
    setNewLabel(''); setShowNewLabel(false); fetchLabels()
  }
  async function delLabel(id: string) {
    const ok = await confirm({ title: t('del_label_title'), message: t('del_label_msg'), confirmLabel: t('delete'), variant: 'danger' })
    if (!ok) return
    await contactsApi.deleteLabel(id); fetchLabels()
    if (view === 'label' && activeLabelId === id) setView('all')
  }
  async function delGroup(id: string) {
    const ok = await confirm({ title: t('contacts_delete_group_title'), message: t('contacts_delete_group_msg'), confirmLabel: t('delete'), variant: 'danger' })
    if (!ok) return
    await contactsApi.deleteGroup(id); fetchGroups()
    if (view === 'group' && activeGroupId === id) setView('all')
  }

  const Nav = (p: { label: string; icon: React.ReactNode; active: boolean; to?: string; onClick?: () => void; badge?: number }) =>
    <SidebarNavItem collapsed={collapsed} {...p} />

  // Row layout shared by labels and groups: the row itself is a plain container
  // carrying the hover/active skin, the navigation is a real <Link> filling it,
  // and the delete affordance is a SIBLING anchor — never an <a> inside an <a>,
  // never a <button> in the sidebar.
  const rowCls = (active: boolean) =>
    `group flex items-center gap-3 px-3 py-2 rounded-full text-sm transition-colors ${
      active ? 'bg-primary-light text-primary' : 'text-text-secondary hover:bg-surface-2'}`
  const rowLinkCls =
    'flex items-center gap-3 flex-1 min-w-0 text-inherit no-underline cursor-pointer rounded-full outline-none focus-visible:ring-2 focus-visible:ring-primary'
  const rowDelCls =
    'hidden group-hover:flex items-center text-text-secondary hover:text-danger cursor-pointer no-underline outline-none focus-visible:ring-2 focus-visible:ring-primary rounded'

  return (
    <>
      {/* "Create a contact" is a pure action (opens the editor), so it is an
          anchor-button: href="#", Space wired by hand, Enter native. */}
      {collapsed ? (
        <div className="flex justify-center mb-3">
          <a href="#" role="button" title={t('create_contact')}
            onClick={e => { e.preventDefault(); setEditorOpen(true) }}
            onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); setEditorOpen(true) } }}
            className="w-10 h-10 flex items-center justify-center bg-white rounded-full transition-shadow cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-primary"
            style={{ boxShadow: '0 1px 3px rgba(60,64,67,0.3), 0 4px 8px rgba(60,64,67,0.15)' }}>
            <Plus size={20} className="text-text-secondary" />
          </a>
        </div>
      ) : (
        <div className="px-3 mb-3">
          <a href="#" role="button"
            onClick={e => { e.preventDefault(); setEditorOpen(true) }}
            onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); setEditorOpen(true) } }}
            className="flex items-center gap-2 bg-white text-sm font-medium text-text-primary no-underline cursor-pointer w-full hover:shadow-md transition-shadow outline-none focus-visible:ring-2 focus-visible:ring-primary"
            style={{ padding: '20px 25px', border: '1px solid #e0e0e0', borderRadius: '20px', boxShadow: '0 1px 3px rgba(0,0,0,0.12)' }}>
            <Plus size={20} className="text-text-secondary" />{t('create_contact')}
          </a>
        </div>
      )}

      <nav className={`flex-1 overflow-y-auto space-y-0.5 ${collapsed ? 'px-2' : 'px-3'}`}>
        <Nav label={t('title_all')} icon={<Users size={20} />} active={view === 'all'} to={hashTo('all')} badge={total} />
        <Nav label={t('title_starred')} icon={<Star size={20} />} active={view === 'starred'} to={hashTo('starred')} />

        {/* Smart views */}
        {collapsed ? <div className="mx-1 my-1 h-px bg-border" /> : (
          <div className="pt-4 pb-1 px-1"><span className="text-xs font-semibold text-text-secondary">{t('smart_views')}</span></div>
        )}
        <Nav label={t('nav_birthdays')} icon={<Cake size={20} />} active={view === 'birthdays'} to={hashTo('birthdays')} />
        <Nav label={t('nav_reminders')} icon={<Bell size={20} />} active={view === 'reminders'} to={hashTo('reminders')} badge={dueCount || undefined} />
        <Nav label={t('nav_frequent')} icon={<Clock size={20} />} active={view === 'frequent'} to={hashTo('frequent')} />
        <Nav label={t('nav_followup')} icon={<RotateCcw size={20} />} active={view === 'followup'} to={hashTo('followup')} />

        {/* Manage */}
        {collapsed ? <div className="mx-1 my-1 h-px bg-border" /> : (
          <div className="pt-4 pb-1 px-1"><span className="text-xs font-semibold text-text-secondary">{t('manage')}</span></div>
        )}
        <Nav label={t('title_duplicates')} icon={<GitMerge size={20} />} active={view === 'duplicates'} to={hashTo('duplicates')} />
        {/* Import opens a dialog: pure action, no addressable view behind it. */}
        <Nav label={t('import')} icon={<Download size={20} />} active={false} onClick={() => setImportOpen(true)} />
        <Nav label={t('nav_archived')} icon={<Archive size={20} />} active={view === 'archived'} to={hashTo('archived')} />
        <Nav label={t('title_trashed')} icon={<Trash2 size={20} />} active={view === 'trashed'} to={hashTo('trashed')} />
        <Nav label={t('nav_settings')} icon={<Settings size={20} />} active={view === 'settings'} to={hashTo('settings')} />

        {/* Labels */}
        {!collapsed && (
          <>
            <div className="pt-4 pb-1 px-1 flex items-center justify-between">
              <span className="text-xs font-semibold text-text-secondary">{t('labels_section')}</span>
              <a href="#" role="button" title={t('create')}
                onClick={e => { e.preventDefault(); setShowNewLabel(v => !v) }}
                onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); setShowNewLabel(v => !v) } }}
                className="p-1 rounded-full hover:bg-surface-2 text-text-secondary cursor-pointer no-underline outline-none focus-visible:ring-2 focus-visible:ring-primary"><Plus size={14} /></a>
            </div>
            {showNewLabel && (
              <div className="px-1 mb-2">
                <Input autoFocus type="text" value={newLabel} onChange={e => setNewLabel(e.target.value)}
                  onKeyDown={e => { if (e.key === 'Enter') createLabel(); if (e.key === 'Escape') setShowNewLabel(false) }}
                  placeholder={t('label_name')} className="mb-1" />
                <a href="#" role="button"
                  onClick={e => { e.preventDefault(); createLabel() }}
                  onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); createLabel() } }}
                  className="text-xs text-primary hover:underline cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-primary">{t('create')}</a>
              </div>
            )}
            {labels.map(l => {
              const active = view === 'label' && activeLabelId === l.id
              return (
                <div key={l.id} className={rowCls(active)}>
                  <RouterLink to={hashTo('label', l.id)} className={rowLinkCls}>
                    <Tag size={16} style={{ color: l.color }} className="flex-shrink-0" />
                    <span className="flex-1 text-left truncate">{l.name}</span>
                    {l.contact_count > 0 && <span className="text-xs opacity-60">{l.contact_count}</span>}
                  </RouterLink>
                  <a href="#" role="button" title={t('delete')}
                    onClick={e => { e.preventDefault(); delLabel(l.id) }}
                    onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); delLabel(l.id) } }}
                    className={rowDelCls}><Trash size={12} /></a>
                </div>
              )
            })}

            {/* Groups */}
            <div className="pt-4 pb-1 px-1"><span className="text-xs font-semibold text-text-secondary">{t('groups')}</span></div>
            {groups.map(g => {
              const active = view === 'group' && activeGroupId === g.id
              return (
                <div key={g.id} className={rowCls(active)}>
                  <RouterLink to={hashTo('group', g.id)} className={rowLinkCls}>
                    <span className="w-4 h-4 rounded-sm flex-shrink-0" style={{ backgroundColor: g.color }} />
                    <span className="flex-1 text-left truncate">{g.name}</span>
                    {g.contact_count > 0 && <span className="text-xs opacity-60">{g.contact_count}</span>}
                  </RouterLink>
                  <a href="#" role="button" title={t('delete')}
                    onClick={e => { e.preventDefault(); delGroup(g.id) }}
                    onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); delGroup(g.id) } }}
                    className={rowDelCls}><Trash size={12} /></a>
                </div>
              )
            })}
          </>
        )}
      </nav>

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </>
  )
}
