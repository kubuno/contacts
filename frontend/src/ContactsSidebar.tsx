import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Users, Star, Trash2, Building2, Plus, Trash } from 'lucide-react'
import { useContactsStore } from './store'
import { contactsApi } from './api'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog, Input } from '@ui'

export default function ContactsSidebar() {
  const { t } = useTranslation('contacts')
  const { view, groups, setView, fetchGroups, total } = useContactsStore()
  const [newGroupName, setNewGroupName] = useState('')
  const [showNewGroup, setShowNewGroup] = useState(false)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  async function handleCreateGroup() {
    if (!newGroupName.trim()) return
    await contactsApi.createGroup(newGroupName.trim())
    setNewGroupName('')
    setShowNewGroup(false)
    fetchGroups()
  }

  async function handleDeleteGroup(e: React.MouseEvent, groupId: string) {
    e.stopPropagation()
    const ok = await confirm({
      title:        t('contacts_delete_group_title'),
      message:      t('contacts_delete_group_msg'),
      confirmLabel: t('common_delete'),
      variant:      'danger',
    })
    if (!ok) return
    await contactsApi.deleteGroup(groupId)
    fetchGroups()
    if (view === 'group') setView('all')
  }

  const navItem = (label: string, icon: React.ReactNode, active: boolean, onClick: () => void, count?: number) => (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-3 px-4 py-2 text-sm rounded-lg transition-colors ${active ? 'bg-primary-light text-primary font-medium' : 'text-text-primary hover:bg-surface-2'}`}
    >
      <span className={active ? 'text-primary' : 'text-text-secondary'}>{icon}</span>
      <span className="flex-1 text-left">{label}</span>
      {count !== undefined && count > 0 && (
        <span className="text-xs text-text-secondary">{count}</span>
      )}
    </button>
  )

  return (
    <div className="w-56 flex-shrink-0 border-r border-border bg-surface-1 flex flex-col overflow-hidden">
      <div className="p-3 space-y-0.5">
        {navItem(t('contacts_nav_all'), <Users size={16} />, view === 'all', () => setView('all'), total)}
        {navItem(t('contacts_nav_starred'), <Star size={16} />, view === 'starred', () => setView('starred'))}
        {navItem(t('contacts_nav_directory'), <Building2 size={16} />, view === 'directory', () => setView('directory'))}
        {navItem(t('contacts_nav_trash'), <Trash2 size={16} />, view === 'trashed', () => setView('trashed'))}
      </div>

      <div className="px-3 mt-2">
        <div className="flex items-center justify-between px-1 mb-1">
          <span className="text-xs font-semibold text-text-secondary uppercase tracking-wide">{t('groups')}</span>
          <button onClick={() => setShowNewGroup(v => !v)} className="p-0.5 rounded hover:bg-surface-2 text-text-secondary">
            <Plus size={14} />
          </button>
        </div>

        {showNewGroup && (
          <div className="mb-2">
            <Input
              autoFocus
              type="text"
              value={newGroupName}
              onChange={e => setNewGroupName(e.target.value)}
              onKeyDown={e => { if (e.key === 'Enter') handleCreateGroup(); if (e.key === 'Escape') setShowNewGroup(false) }}
              placeholder={t('group_name')}
              className="mb-1"
            />
            <button onClick={handleCreateGroup} className="text-xs text-primary hover:underline">{t('create')}</button>
          </div>
        )}

        <div className="space-y-0.5">
          {groups.map(g => (
            <button
              key={g.id}
              onClick={() => setView('group', g.id)}
              className={`w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-lg transition-colors group ${view === 'group' && useContactsStore.getState().activeGroupId === g.id ? 'bg-primary-light text-primary' : 'text-text-primary hover:bg-surface-2'}`}
            >
              <span className="w-2 h-2 rounded-full flex-shrink-0" style={{ backgroundColor: g.color }} />
              <span className="flex-1 text-left truncate">{g.name}</span>
              <span className="text-xs text-text-secondary">{g.contact_count}</span>
              <button
                onClick={e => handleDeleteGroup(e, g.id)}
                className="hidden group-hover:block text-text-secondary hover:text-danger ml-1"
              >
                <Trash size={12} />
              </button>
            </button>
          ))}
        </div>
      </div>
      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  )
}
