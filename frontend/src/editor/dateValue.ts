import type { DateValue } from '@ui'

/* Conversion between the split DateField value (day / month / year, the year
 * optional) and the string the API stores. The backend's parser accepts
 * `YYYY-MM-DD` and the vCard `--MM-DD` form for a date whose year is unknown —
 * a birthday you celebrate without knowing the age. */

export const EMPTY_DATE: DateValue = { day: '', month: '', year: '' }

export function parseDateValue(raw: string | undefined | null): DateValue {
  const t = (raw ?? '').trim()
  if (!t) return { ...EMPTY_DATE }
  const noYear = t.match(/^--(\d{1,2})-(\d{1,2})$/)
  if (noYear) return { day: String(Number(noYear[2])), month: String(Number(noYear[1])), year: '' }
  const full = t.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/)
  if (full) return { day: String(Number(full[3])), month: String(Number(full[2])), year: full[1] }
  return { ...EMPTY_DATE }
}

/** `null` when the date holds nothing usable (a day/month pair is the minimum). */
export function formatDateValue(v: DateValue): string | null {
  const d = Number(v.day), m = Number(v.month)
  if (!d || !m) return null
  const dd = String(d).padStart(2, '0')
  const mm = String(m).padStart(2, '0')
  return v.year.trim().length === 4 ? `${v.year}-${mm}-${dd}` : `--${mm}-${dd}`
}

export function isDateEmpty(v: DateValue): boolean {
  return !v.day.trim() && !v.month.trim() && !v.year.trim()
}
