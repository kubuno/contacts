import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'
import { useContactsStore, type View } from '../store'
import { fromHash } from '../hashRoute'

// Views reachable through a sidebar HASH link (#starred, #group/<id>…). The
// desktop sidebar encodes every view this way so each has a shareable URL.
const HASH_VIEWS: readonly string[] = [
  'all', 'starred', 'birthdays', 'reminders', 'frequent', 'followup',
  'duplicates', 'archived', 'trashed', 'settings', 'group', 'label',
  'other', 'unit',
]

// A handful of views also have a REAL sub-route, so the mobile bottom nav can
// point at them: NavLink derives its active state from the pathname, which a
// hash cannot drive (every hash shares the `/contacts` pathname).
const PATH_VIEWS: Record<string, View> = {
  '/contacts':           'all',
  '/contacts/starred':   'starred',
  '/contacts/trashed':   'trashed',
  '/contacts/directory': 'directory',
}

/**
 * Single source of truth for the active view, derived from the URL. Replaces the
 * effect that used to live in the sidebar so a click, a deep link and the Back
 * button all converge here.
 *
 * Precedence: a module HASH wins (the sidebar's shareable links, including the
 * group/label ids); otherwise the pathname's sub-route (the mobile bottom-nav
 * destinations); a bare `/contacts` with no hash is the "all" view. Keeping the
 * hash authoritative means the two navigation styles never fight.
 */
export function useContactsRoute(): void {
  const { pathname, hash } = useLocation()
  const setView = useContactsStore(s => s.setView)
  useEffect(() => {
    const h = fromHash(hash)
    if (h && HASH_VIEWS.includes(h.kind)) {
      setView(h.kind as View, h.id ?? undefined)
      return
    }
    const v = PATH_VIEWS[pathname]
    if (v) setView(v)
  }, [pathname, hash, setView])
}
