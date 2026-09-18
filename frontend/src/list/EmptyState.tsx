import { useTranslation } from 'react-i18next'
import { UserPlus, Download } from 'lucide-react'

function EmptyIllustration() {
  return (
    <svg width="160" height="160" viewBox="0 0 180 180" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M68 128 Q62 150 56 165 L124 165 Q118 150 112 128 Z" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5" strokeLinejoin="round"/>
      <rect x="72" y="118" width="36" height="12" rx="5" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="90" cy="40" r="16" fill="#e8f0fe" stroke="#1a73e8" strokeWidth="1.5"/>
      <circle cx="90" cy="35" r="6" fill="#1a73e8"/>
      <path d="M79 52 Q90 48 101 52" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
      <line x1="40" y1="165" x2="140" y2="165" stroke="#1a73e8" strokeWidth="1.5" strokeLinecap="round"/>
    </svg>
  )
}

/** Shown when the current view holds no contact: the illustration and the two
 *  ways in (create, import). */
export default function EmptyState({ onCreate, onImport }: { onCreate: () => void; onImport: () => void }) {
  const { t } = useTranslation('contacts')
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-4 px-6">
      <EmptyIllustration />
      <p className="text-base text-text-primary">{t('empty')}</p>
      <div className="flex items-center gap-8">
        <button onClick={onCreate} className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"><UserPlus size={16} />{t('create_contact')}</button>
        <button onClick={onImport} className="flex items-center gap-2 text-sm font-medium text-primary hover:underline"><Download size={16} />{t('import_contacts')}</button>
      </div>
    </div>
  )
}
