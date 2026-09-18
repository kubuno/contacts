import { COUNTRIES, countryOf, type PhoneValue } from '@ui'

/* Conversion between the structured PhoneField value (ISO country + local
 * number) and the single string the API stores. Storage stays a plain string —
 * that is what vCard, CardDAV and every consumer already read — so the country
 * is folded back into the international prefix on save.
 *
 * Both directions read the SAME country table the selector uses, so a number
 * typed in the field and one read back from the server resolve identically. */

export const DEFAULT_COUNTRY = 'FR'

// Longest dial code first: +352 (Luxembourg) must win over +35, +33 over +3.
const BY_DIAL_DESC = [...COUNTRIES].sort((a, b) => b.dial.length - a.dial.length)

export function parsePhoneValue(raw: string | undefined | null, label?: string): PhoneValue {
  const t = (raw ?? '').replace(/\s+/g, ' ').trim()
  if (t.startsWith('+')) {
    const digits = t.slice(1)
    for (const c of BY_DIAL_DESC) {
      if (digits.startsWith(c.dial)) {
        return { country: c.iso2, number: digits.slice(c.dial.length).trim(), label }
      }
    }
  }
  return { country: DEFAULT_COUNTRY, number: t, label }
}

/** The stored string, in international form. Empty when there is no number. */
export function formatPhoneValue(v: PhoneValue): string {
  const n = v.number.trim()
  if (!n) return ''
  if (n.startsWith('+')) return n
  const dial = countryOf(v.country)?.dial
  if (!dial) return n
  // A local number often keeps its trunk "0" (06…): it is dropped once the
  // country code is in front, as +33 6… — not +33 06…
  return `+${dial}${n.startsWith('0') ? n.slice(1) : n}`
}

export function isPhoneEmpty(v: PhoneValue): boolean {
  return !v.number.trim()
}
