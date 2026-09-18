/**
 * The two ways the editor offers to add something, following Google Contacts:
 *
 *  · the section is EMPTY  → a full-width tinted button with its icon. It has to
 *    carry the affordance on its own, since nothing above it hints at the field.
 *  · the section already HAS a row → a discreet blue link under the last row,
 *    aligned on the field column. The rows above already say what this is.
 */
export default function AddAction({ label, icon, filled, onClick }: {
  label:   string
  icon?:   React.ReactNode
  /** `true` → the tinted full-width button (empty section). */
  filled:  boolean
  onClick: () => void
}) {
  if (filled) {
    return (
      <button type="button" onClick={onClick}
        className="w-full flex items-center justify-center gap-2 py-3 rounded-lg
                   bg-primary-light/60 hover:bg-primary-light text-primary text-sm font-medium
                   transition-colors">
        {icon}{label}
      </button>
    )
  }
  return (
    // Indented onto the field column (20px gutter + 12px gap) so the link starts
    // where the values above it start.
    <div className="pl-8">
      <button type="button" onClick={onClick}
        className="text-sm text-primary hover:underline">
        {label}
      </button>
    </div>
  )
}
