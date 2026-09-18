import { useEffect, useState } from 'react'

/**
 * Subscribes to a media query in JS rather than through a `lg:` class.
 *
 * A module bundle cannot rely on responsive variants that CANCEL a base class
 * (`w-full` → `lg:w-56`): the host's own utility layer wins the cascade
 * (utilities > kubuno-module), so the wide rule never applies. Reading the query
 * here and branching in the markup is the only reliable way from inside a
 * module.
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() =>
    typeof window !== 'undefined' && window.matchMedia(query).matches)
  useEffect(() => {
    const mq = window.matchMedia(query)
    const on = () => setMatches(mq.matches)
    on()
    mq.addEventListener('change', on)
    return () => mq.removeEventListener('change', on)
  }, [query])
  return matches
}

/** Below the shell's mobile breakpoint. */
export const useIsMobile = () => useMediaQuery('(max-width: 1023px)')

/**
 * Wide enough to lay a form out in two columns. The threshold counts the space
 * the FORM gets, not the window: the shell already spends ~264px on its sidebar
 * and ~56px on the right rail, so two 480px columns need roughly this much.
 */
export const useIsWide = () => useMediaQuery('(min-width: 1400px)')
