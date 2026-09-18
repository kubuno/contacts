interface ContactsLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Contacts logo (designer artwork, raster). Served by the host from
 *  `/contacts-logo.png`; rendered as a square image so it weighs the same as
 *  its neighbours in the waffle menu. */
export function ContactsLogo({ size = 24, className, title = 'Contacts' }: ContactsLogoProps) {
  return (
    <img
      src="/contacts-logo.png"
      width={size}
      height={size}
      alt={title}
      className={className}
      style={{ display: 'block', objectFit: 'contain' }}
    />
  )
}

export default ContactsLogo
