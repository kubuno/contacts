import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { MoreHorizontal, Download, Upload, ArrowDownUp, Filter, List, LayoutGrid, Table as TableIcon } from 'lucide-react'
import { MenuDropdown, type MenuItem, type MenuDropdownPos } from '@ui'
import { useContactsStore, type ViewMode } from '../store'
import { contactsApi } from '../api'

/**
 * The list controls: sort, filter, the list/grid/table switch and the overflow
 * (export/import) menu. The table mode is a desktop affair — a phone screen has
 * no room for it, so on mobile the switch is list/grid only and the current mode
 * is coerced away from `table`.
 */
export default function ListToolbar({ isMobile }: { isMobile: boolean }) {
  const { t } = useTranslation('contacts')
  const s = useContactsStore()
  const { viewMode, sort, setViewMode, setSort, setFilter, setImportOpen } = s
  const [menu, setMenu] = useState<{ pos: MenuDropdownPos; items: MenuItem[] } | null>(null)

  function openSortMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const opt = (key: string, label: string): MenuItem => ({ type: 'action', label, checked: sort === key, onClick: () => setSort(key) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: [
      opt('name', t('sort_name')), opt('name_desc', t('sort_name_desc')),
      opt('recent', t('sort_recent')), opt('updated', t('sort_updated')),
      opt('organization', t('sort_org')), opt('last_interaction', t('sort_interaction')),
    ]})
  }
  function openFilterMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const opt = (key: string | null, label: string): MenuItem => ({ type: 'action', label, checked: s.filter === key, onClick: () => setFilter(key) })
    setMenu({ pos: { top: r.bottom + 4, left: r.left }, items: [
      opt(null, t('filter_all')), { type: 'separator' },
      opt('incomplete', t('filter_incomplete')),
      opt('missing_email', t('filter_no_email')), opt('missing_phone', t('filter_no_phone')),
      opt('no_group', t('filter_no_group')), opt('no_label', t('filter_no_label')),
    ]})
  }
  function openActionsMenu(e: React.MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setMenu({ pos: { top: r.bottom + 4, left: r.left - 120 }, items: [
      { type: 'action', label: t('set_export_vcf'), icon: <Download size={15} />, onClick: () => contactsApi.exportVcf({}) },
      { type: 'action', label: t('set_export_csv'), icon: <Download size={15} />, onClick: () => contactsApi.exportCsv({}) },
      { type: 'action', label: t('import'), icon: <Upload size={15} />, onClick: () => setImportOpen(true) },
    ]})
  }

  const ToolBtn = ({ icon, label, onClick, active }: { icon: React.ReactNode; label: string; onClick: (e: React.MouseEvent) => void; active?: boolean }) => (
    <button onClick={onClick} title={label}
      className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-sm transition-colors ${active ? 'bg-primary-light text-primary' : 'hover:bg-surface-2 text-text-secondary'}`}>
      {icon}<span className="hidden lg:inline">{label}</span>
    </button>
  )
  const modeBtn = (m: ViewMode, icon: React.ReactNode) => (
    <button onClick={() => setViewMode(m)} title={m}
      className={`p-1.5 rounded-lg transition-colors ${viewMode === m ? 'bg-primary-light text-primary' : 'hover:bg-surface-2 text-text-secondary'}`}>{icon}</button>
  )

  return (
    <div className={`${isMobile ? 'px-4' : 'px-6'} pb-1 flex items-center gap-1 flex-shrink-0 no-print`}>
      <ToolBtn icon={<ArrowDownUp size={15} />} label={t('sort_by')} onClick={openSortMenu} />
      <ToolBtn icon={<Filter size={15} />} label={t('filter_label')} onClick={openFilterMenu} active={!!s.filter} />
      <div className="flex-1" />
      <div className="flex items-center gap-0.5 bg-surface-1 rounded-lg p-0.5">
        {modeBtn('list', <List size={16} />)}
        {modeBtn('grid', <LayoutGrid size={16} />)}
        {/* Table is a desktop-only density — no room on a phone. */}
        {!isMobile && modeBtn('table', <TableIcon size={16} />)}
      </div>
      <button onClick={openActionsMenu} className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary"><MoreHorizontal size={16} /></button>
      {menu && <MenuDropdown items={menu.items} pos={menu.pos} onClose={() => setMenu(null)} />}
    </div>
  )
}
