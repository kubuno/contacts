/**
 * Store backing the globally-mounted `ContactPickerDialog` (slot `app-dialogs`).
 *
 * Same pattern as drive's `filesDialogStore`: the promise returned by
 * `pickContact()` is parked in the store and settled by `_resolve()` when the
 * dialog closes. This lets ANY module (chat…) obtain a contact envelope through
 * `ModuleServiceRegistry.call('contacts', 'pickContact')` without ever importing
 * contacts' code — when contacts is not installed, the service is simply absent.
 */
import { create } from 'zustand'
import type { KubunoDataEnvelope } from './kubunoData'

export interface ContactPickerOptions {
  /** Optional dialog title override (consumers may pass their own wording). */
  title?: string
}

interface ContactPickerState {
  pickerOpts: ContactPickerOptions | null
  /** Settles the pending `pickContact()` promise and closes the dialog. */
  _resolve: (envelope: KubunoDataEnvelope | null) => void
  /** Opens the picker; resolves with the chosen contact's envelope, or null. */
  pickContact: (opts?: ContactPickerOptions) => Promise<KubunoDataEnvelope | null>
}

export const useContactPickerStore = create<ContactPickerState>((set, get) => {
  let pending: ((envelope: KubunoDataEnvelope | null) => void) | null = null

  return {
    pickerOpts: null,

    _resolve: (envelope) => {
      const resolve = pending
      pending = null
      set({ pickerOpts: null })
      resolve?.(envelope)
    },

    pickContact: (opts = {}) => {
      // A second call supersedes a pending one: cancel it rather than leak it.
      if (pending) get()._resolve(null)
      return new Promise<KubunoDataEnvelope | null>((resolve) => {
        pending = resolve
        set({ pickerOpts: opts })
      })
    },
  }
})

/** Convenience wrapper published on the `contacts` service registry. */
export function pickContact(opts?: ContactPickerOptions): Promise<KubunoDataEnvelope | null> {
  return useContactPickerStore.getState().pickContact(opts)
}
