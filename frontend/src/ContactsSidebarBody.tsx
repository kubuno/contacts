import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Users, Clock, UserMinus, GitMerge, Download, Trash2, Plus, Trash,
} from 'lucide-react'
import { useContactsStore } from './store'
import { contactsApi } from './api'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog, Input } from '@ui'
import { SidebarNavItem } from '@kubuno/sdk'

export default function ContactsSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('contacts')
  const {
    view, groups, total, setView, fetchGroups,
    setEditorOpen, setImportOpen,
  } = useContactsStore()
  const [newGroupName, setNewGroupName]   = useState('')
  const [showNewGroup, setShowNewGroup]   = useState(false)
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
      title:        t('del_label_title'),
      message:      t('del_label_msg'),
      confirmLabel: t('delete'),
      variant:      'danger',
    })
    if (!ok) return
    await contactsApi.deleteGroup(groupId)
    fetchGroups()
    if (view === 'group') setView('all')
  }

  const activeGroupId = useContactsStore(s => s.activeGroupId)

  function NavItem({
    label, icon, active, onClick, badge,
  }: {
    label:   string
    icon:    React.ReactNode
    active:  boolean
    onClick: () => void
    badge?:  number
  }) {
    return (
      <SidebarNavItem collapsed={collapsed} label={label} icon={icon}
        active={active} onClick={onClick} badge={badge} />
    )
  }

  return (
    <>
      {/* Créer un contact (compact en mode replié) */}
      {collapsed ? (
        <div className="flex justify-center mb-3">
          <button
            onClick={() => setEditorOpen(true)}
            title={t('create_contact')}
            className="w-10 h-10 flex items-center justify-center bg-white rounded-full transition-shadow"
            style={{ boxShadow: '0 1px 3px rgba(60,64,67,0.3), 0 4px 8px rgba(60,64,67,0.15)' }}
          >
            <Plus size={20} className="text-text-secondary" />
          </button>
        </div>
      ) : (
        <div className="px-3 mb-3">
          <button
            onClick={() => setEditorOpen(true)}
            className="flex items-center gap-2 bg-white text-sm font-medium text-text-primary
                       cursor-pointer w-full hover:shadow-md transition-shadow"
            style={{
              padding:      '20px 25px',
              border:       '1px solid #e0e0e0',
              borderRadius: '20px',
              boxShadow:    '0 1px 3px rgba(0,0,0,0.12)',
            }}
          >
            <Plus size={20} className="text-text-secondary" />
            {t('create_contact')}
          </button>
        </div>
      )}

      {/* Primary nav */}
      <nav className={`flex-1 overflow-y-auto space-y-0.5 ${collapsed ? 'px-2' : 'px-3'}`}>
        <NavItem
          label={t('title_all')}
          icon={<Users size={20} />}
          active={view === 'all'}
          onClick={() => setView('all')}
          badge={total}
        />
        <NavItem
          label={t('title_starred')}
          icon={<Clock size={20} />}
          active={view === 'starred'}
          onClick={() => setView('starred')}
        />
        <NavItem
          label={t('other_contacts')}
          icon={<UserMinus size={20} />}
          active={false}
          onClick={() => {}}
        />

        {/* Corriger et gérer */}
        {collapsed ? <div className="mx-1 my-1 h-px bg-border" /> : (
          <div className="pt-4 pb-1 px-1">
            <span className="text-xs font-semibold text-text-secondary">{t('manage')}</span>
          </div>
        )}
        <NavItem
          label={t('title_duplicates')}
          icon={<GitMerge size={20} />}
          active={view === 'duplicates'}
          onClick={() => setView('duplicates')}
        />
        <NavItem
          label={t('import')}
          icon={<Download size={20} />}
          active={false}
          onClick={() => setImportOpen(true)}
        />
        <NavItem
          label={t('title_trashed')}
          icon={<Trash2 size={20} />}
          active={view === 'trashed'}
          onClick={() => setView('trashed')}
        />

        {/* Libellés (masqués en mode replié) */}
        {!collapsed && (
        <div className="pt-4 pb-1 px-1 flex items-center justify-between">
          <span className="text-xs font-semibold text-text-secondary">{t('labels')}</span>
          <button
            onClick={() => setShowNewGroup(v => !v)}
            className="p-1 rounded-full hover:bg-surface-2 text-text-secondary transition-colors"
          >
            <Plus size={14} />
          </button>
        </div>
        )}

        {!collapsed && showNewGroup && (
          <div className="px-1 mb-2">
            <Input
              autoFocus
              type="text"
              value={newGroupName}
              onChange={e => setNewGroupName(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') handleCreateGroup()
                if (e.key === 'Escape') setShowNewGroup(false)
              }}
              placeholder={t('label_name')}
              className="mb-1"
            />
            <button onClick={handleCreateGroup} className="text-xs text-primary hover:underline">
              {t('create')}
            </button>
          </div>
        )}

        {!collapsed && groups.map(g => {
          const isActive = view === 'group' && activeGroupId === g.id
          return (
            <button
              key={g.id}
              onClick={() => setView('group', g.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-full text-sm transition-colors group
                          ${isActive
                            ? 'bg-primary-light text-primary'
                            : 'text-text-secondary hover:bg-surface-2'
                          }`}
            >
              <span
                className="w-4 h-4 rounded-sm flex-shrink-0"
                style={{ backgroundColor: g.color }}
              />
              <span className="flex-1 text-left truncate">{g.name}</span>
              {g.contact_count > 0 && (
                <span className="text-xs opacity-60">{g.contact_count}</span>
              )}
              <button
                onClick={e => handleDeleteGroup(e, g.id)}
                className="hidden group-hover:flex items-center text-text-secondary hover:text-danger"
              >
                <Trash size={12} />
              </button>
            </button>
          )
        })}
      </nav>

      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </>
  )
}
