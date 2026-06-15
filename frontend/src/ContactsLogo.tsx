interface ContactsLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Logo Contacts : carré arrondi bleu + silhouette blanche (tête et épaules). */
export function ContactsLogo({ size = 24, className, title = 'Contacts' }: ContactsLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 512 512"
      fill="none"
      role="img"
      aria-label={title}
      className={className}
    >
      <title>{title}</title>
      <rect width="512" height="512" rx="114" fill="#2563EB" />
      <circle cx="256" cy="196" r="68" fill="#FFFFFF" />
      <path d="M128 384 C128 312 200 292 256 292 C312 292 384 312 384 384 Z" fill="#FFFFFF" />
    </svg>
  )
}

export default ContactsLogo
