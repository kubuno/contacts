import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'

/* Height of ONE field box, shared with the @ui primitives (`FieldGroup.rowH`,
 * `PhoneField.boxH`). The icon and the remove button are centred inside a band
 * of exactly this height at the TOP of the row, so on a block of several
 * stacked fields — a postal address — they sit on the first field instead of
 * drifting to the middle of the stack. */
const ROW_H = 48

/**
 * One row of the editor: the icon gutter, the field itself, and a remove cross.
 *
 * The gutter is a fixed 20px + the 12px gap `FieldGroup` uses internally, so a
 * plain row and a grouped row (Name, Organisation) put their fields on the very
 * same left edge. Only the FIRST row of a section carries the icon; the others
 * keep an empty gutter so the column stays true.
 *
 * The cross appears on hover / keyboard focus like Google's, never permanently —
 * a delete affordance on every row at rest turns a form into a minefield.
 */
export default function FieldRow({ icon, onRemove, children }: {
  icon?:     React.ReactNode
  /** Omitted for a row that cannot be removed (the last one of a required set). */
  onRemove?: () => void
  children:  React.ReactNode
}) {
  const { t } = useTranslation('contacts')
  const band = 'flex-shrink-0 flex items-center justify-center'
  return (
    <div className="group flex items-start gap-3">
      <span className={`w-5 text-text-secondary ${band}`} style={{ height: ROW_H }}>{icon}</span>
      <div className="flex-1 min-w-0">{children}</div>
      <span className={`w-8 ${band}`} style={{ height: ROW_H }}>
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            title={t('common_delete')}
            aria-label={t('common_delete')}
            className="w-8 h-8 rounded-full flex items-center justify-center text-text-secondary
                       opacity-0 group-hover:opacity-100 focus-visible:opacity-100
                       hover:bg-surface-2 transition-opacity"
          >
            <X size={18} />
          </button>
        )}
      </span>
    </div>
  )
}
