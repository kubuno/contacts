import { createContext, useContext, useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Edit2, Star, StarOff, Tag, Users, Share2, Download, Archive, ArchiveRestore,
  Ban, Trash2, RotateCcw, Bell,
} from 'lucide-react'
import { MenuDropdown, type MenuItem, type MenuDropdownPos } from '@ui'
import { Contact, contactsApi } from './api'
import { useContactsStore } from './store'
import ShareModal from './ShareModal'

interface Ctx { open: (e: React.MouseEvent, contact: Contact) => void }
const ContactMenuContext = createContext<Ctx>({ open: () => {} })
export const useContactMenu = () => useContext(ContactMenuContext)

/** Provides a right-click context menu for contacts, anywhere in the subtree. */
export function ContactMenuProvider({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation('contacts')
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)
  const [shareId, setShareId] = useState<string | null>(null)

  const open = useCallback((e: React.MouseEvent, contact: Contact) => {
    e.preventDefault()
    e.stopPropagation()
    const s = useContactsStore.getState()
    const { selectedIds, labels, groups, view, fetchContacts, fetchLabels, setSelectedId, setEditorOpen } = s
    // Act on the whole selection when right-clicking a selected contact.
    const targets = selectedIds.has(contact.id) && selectedIds.size > 1 ? [...selectedIds] : [contact.id]
    const multi = targets.length > 1

    const refresh = () => { fetchContacts(); fetchLabels() }
    const bulk = async (action: string) => { await contactsApi.bulk(targets, action); s.clearSelection(); fetchContacts() }

    const items: MenuItem[] = []

    if (view === 'trashed') {
      items.push(
        { type: 'action', label: t('contacts_restore'), icon: <RotateCcw size={15} />, onClick: () => bulk('restore') },
        { type: 'action', label: t('contacts_delete_perm'), danger: true, icon: <Trash2 size={15} />, onClick: () => bulk('delete') },
      )
      setMenu({ pos: { top: e.clientY, left: e.clientX }, items })
      return
    }

    if (!multi) {
      items.push(
        { type: 'action', label: t('common_edit'), icon: <Edit2 size={15} />, onClick: () => { setSelectedId(contact.id); setEditorOpen(true) } },
      )
    }
    items.push({
      type: 'action',
      label: contact.is_starred && !multi ? t('contacts_unstar') : t('contacts_star'),
      icon: contact.is_starred && !multi ? <StarOff size={15} /> : <Star size={15} />,
      onClick: () => bulk(contact.is_starred && !multi ? 'unstar' : 'star'),
    })

    // Labels submenu (toggle for single; add for multi)
    items.push({
      type: 'submenu', label: t('sel_label'), icon: <Tag size={15} />,
      items: labels.length
        ? labels.map(l => ({
            type: 'action' as const, label: l.name,
            checked: !multi && contact.label_ids?.includes(l.id),
            icon: <span className="w-3 h-3 rounded-full inline-block" style={{ backgroundColor: l.color }} />,
            onClick: async () => {
              const has = !multi && contact.label_ids?.includes(l.id)
              if (has) await contactsApi.removeLabelMembers(l.id, targets)
              else await contactsApi.addLabelMembers(l.id, targets)
              refresh()
            },
          }))
        : [{ type: 'label' as const, text: t('rem_empty') }],
    })

    // Groups submenu
    items.push({
      type: 'submenu', label: t('sel_group'), icon: <Users size={15} />,
      items: groups.length
        ? groups.map(g => ({
            type: 'action' as const, label: g.name,
            icon: <span className="w-3 h-3 rounded-sm inline-block" style={{ backgroundColor: g.color }} />,
            onClick: async () => { await contactsApi.addGroupMembers(g.id, targets); fetchContacts() },
          }))
        : [{ type: 'label' as const, text: t('rem_empty') }],
    })

    if (!multi) {
      items.push(
        { type: 'action', label: t('det_share'), icon: <Share2 size={15} />, onClick: () => setShareId(contact.id) },
        {
          type: 'action', label: t('bday_remind'), icon: <Bell size={15} />,
          onClick: async () => {
            await contactsApi.createReminder({
              contact_id: contact.id, kind: 'follow_up',
              message: contact.display_name,
              remind_at: new Date(Date.now() + 7 * 86400000).toISOString(),
            })
            s.fetchDueCount()
          },
        },
      )
    }

    items.push(
      { type: 'action', label: t('common_export'), icon: <Download size={15} />, onClick: () => contactsApi.exportVcf({}) },
      { type: 'separator' },
      {
        type: 'action',
        label: contact.is_archived && !multi ? t('det_unarchive') : t('det_archive'),
        icon: contact.is_archived && !multi ? <ArchiveRestore size={15} /> : <Archive size={15} />,
        onClick: () => bulk(contact.is_archived && !multi ? 'unarchive' : 'archive'),
      },
    )
    if (!multi) {
      items.push({
        type: 'action',
        label: contact.is_blocked ? t('det_unblock') : t('det_block'),
        icon: <Ban size={15} />,
        onClick: () => bulk(contact.is_blocked ? 'unblock' : 'block'),
      })
    }
    items.push({ type: 'action', label: t('sel_delete'), danger: true, icon: <Trash2 size={15} />, onClick: () => bulk('trash') })

    setMenu({ pos: { top: e.clientY, left: e.clientX }, items })
  }, [t])

  return (
    <ContactMenuContext.Provider value={{ open }}>
      {children}
      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
      {shareId && <ShareModal contactId={shareId} onClose={() => setShareId(null)} />}
    </ContactMenuContext.Provider>
  )
}
