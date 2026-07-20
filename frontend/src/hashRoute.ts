// Contacts views (all, starred, a group, a label…) are client-side state rather
// than routes of their own. They still deserve a real, shareable link, so the
// left sidebar encodes them in the URL HASH: /contacts/#starred,
// /contacts/#group/<id>, /contacts/#label/<id>. Every sidebar entry is then an
// <a> carrying a genuine href, and direct links plus the browser Back button
// keep working because the view is read back from the location hash.

/** Route the module is mounted on. */
export const CONTACTS_BASE = '/contacts'

/** `to=` value for a view link: /contacts/#<kind>[/<id>]. */
export function hashTo(kind: string, id?: string | null): string {
  return `${CONTACTS_BASE}/#${kind}${id ? `/${encodeURIComponent(id)}` : ''}`
}

/**
 * Parse a location hash produced by `hashTo`.
 * Returns null when the hash holds nothing this module owns.
 */
export function fromHash(hash: string): { kind: string; id: string | null } | null {
  const m = /^#([a-z]+)(?:\/([^/#?]+))?$/.exec(hash)
  if (!m) return null
  return { kind: m[1], id: m[2] ? decodeURIComponent(m[2]) : null }
}
