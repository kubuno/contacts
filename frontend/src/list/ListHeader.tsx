import { useTranslation } from 'react-i18next'
import { Search, UserPlus } from 'lucide-react'

/** View title with its count, and the in-view search box. On a phone the title
 *  shrinks, the search takes the full width on its own line, and a "+" appears
 *  top-right for creating a contact — the desktop's sidebar "New" button is in
 *  the drawer on mobile, and the shell already owns the bottom-right FAB slot
 *  (the app-switcher waffle), so a second floating button there would collide. */
export default function ListHeader({ title, total, search, onSearch, isMobile, onCreate }: {
  title:    string
  total:    number
  search:   string
  onSearch: (v: string) => void
  isMobile: boolean
  onCreate: () => void
}) {
  const { t } = useTranslation('contacts')
  return (
    <div className={`${isMobile ? 'px-4' : 'px-6'} pt-4 pb-2 flex items-center flex-wrap gap-x-3 gap-y-2 flex-shrink-0`}>
      <h1 className={`${isMobile ? 'text-xl' : 'text-2xl'} font-normal text-text-primary truncate`}>
        {title}{total > 0 && <span className={`text-text-secondary ml-2 ${isMobile ? 'text-base' : 'text-xl'}`}>({total})</span>}
      </h1>
      <div className="flex-1" />
      {isMobile && (
        <button onClick={onCreate} aria-label={t('create_contact')} title={t('create_contact')}
          className="w-9 h-9 rounded-full flex items-center justify-center bg-primary text-white active:scale-95 transition-transform flex-shrink-0">
          <UserPlus size={18} />
        </button>
      )}
      <div className={`relative no-print ${isMobile ? 'w-full' : 'w-auto'}`}>
        <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary" />
        <input value={search} onChange={e => onSearch(e.target.value)}
          placeholder={t('contacts_search_ph')} title={t('search_help')}
          className={`pl-9 pr-3 py-2 ${isMobile ? 'w-full' : 'w-56'} rounded-full bg-surface-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/30`} />
      </div>
    </div>
  )
}
