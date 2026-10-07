import { useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArrowLeft } from 'lucide-react'
import { Spinner } from '@ui'
import ContactDetail from '../ContactDetail'
import { useIsMobile } from '../hooks/useMediaQuery'
import { contactsApi, type Contact } from '../api'
import { CONTACTS_BASE } from '../hashRoute'

/**
 * A contact on its own page, at `/contacts/person/<id>` — the way Google
 * Contacts does it, and the reason the old 380px band on the right is gone: a
 * contact card is a destination you go to and come back from, not a strip
 * squeezed beside the list. It gets the whole area, it has a real URL to share,
 * and the browser's Back button returns to the list on its own.
 *
 * The record is fetched here rather than read from the list, so a direct link
 * works even when the contact is not part of the current view (a search result,
 * a link from another module).
 */
export default function ContactDetailPage() {
  const { id = '' } = useParams<{ id: string }>()
  const { t } = useTranslation('contacts')
  const navigate = useNavigate()
  const isMobile = useIsMobile()
  const [contact, setContact] = useState<Contact | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let cancelled = false
    setContact(null); setFailed(false)
    contactsApi.getContact(id)
      .then(r => { if (!cancelled) setContact(r.data.contact) })
      .catch(() => { if (!cancelled) setFailed(true) })
    return () => { cancelled = true }
  }, [id])

  // Back goes through history when there is one, so returning lands on the view
  // the contact was opened from (a label, the trash…) rather than always "all".
  const back = () => {
    if (window.history.length > 1) navigate(-1)
    else navigate(CONTACTS_BASE)
  }

  return (
    <div className="h-full overflow-y-auto bg-white">
      <div className={`${isMobile ? 'px-2' : 'px-6'} pt-3 sticky top-0 bg-white z-10`}>
        <button onClick={back} aria-label={t('common_back')}
          className="w-10 h-10 rounded-full flex items-center justify-center hover:bg-surface-2 text-text-secondary">
          <ArrowLeft size={22} />
        </button>
      </div>

      {failed ? (
        <p className="px-6 py-10 text-sm text-text-secondary">{t('no_name')}</p>
      ) : !contact ? (
        <div className="flex justify-center py-16"><Spinner /></div>
      ) : (
        <ContactDetail contact={contact} />
      )}
    </div>
  )
}
