// Instance administration of contacts, rendered in the core admin console under
// Modules ▸ Contacts.
//
// Two bespoke sections sit beside the declared settings:
//   • "directory"   — says where the ACCOUNT directory is governed (the core),
//                     because this module deliberately does not govern it;
//   • "shared_book" — the instance's shared address book, which needs records,
//                     not a toggle: the setting next to it only publishes it.

import { useCallback, useEffect, useState } from 'react'
import { Building2, Pencil, Plus, Trash2, X } from 'lucide-react'
import { ModuleAdminRegistry, useConfirm } from '@kubuno/sdk'
import { Button, Callout, ConfirmDialog, Input } from '@ui'
import { contactsApi, type SharedContact, type SharedContactInput } from '../api'

// ── Account directory: governed by the core, on purpose ────────────────────────

function DirectoryNotice() {
  return (
    <div className="rounded-xl border border-border bg-surface-1 p-5">
      <div className="flex items-start gap-3">
        <Building2 size={20} className="text-primary shrink-0 mt-0.5" />
        <div className="space-y-2 text-sm text-text-secondary">
          <p className="text-text-primary font-medium">L'annuaire des comptes se règle ailleurs</p>
          <p>
            La visibilité de l'annuaire, le partage des adresses de courriel et les champs de
            profil qu'un utilisateur peut modifier lui-même relèvent du <strong>cœur</strong>,
            pas de ce module : ils portent sur les comptes, pas sur les carnets d'adresses.
            Ils se règlent dans <a href="/admin/directory-settings" className="text-primary hover:underline">Annuaire ▸ Paramètres d'annuaire</a>.
          </p>
          <p>
            Ce module ne les redéfinit pas et n'en garde pas de copie : une copie locale
            contournerait cette politique — c'est exactement pourquoi celle qui existait a été
            supprimée. Les pages suivantes ne portent donc que sur ce que le module possède :
            les carnets personnels, les liens de partage, la synchronisation et le carnet
            partagé de l'instance.
          </p>
        </div>
      </div>
    </div>
  )
}

// ── Shared address book ───────────────────────────────────────────────────────

const EMPTY: SharedContactInput = {
  display_name: '', organization: '', job_title: '', email: '', phone: '', notes: '',
}

function SharedBookManager() {
  const [rows, setRows]       = useState<SharedContact[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError]     = useState<string | null>(null)
  const [editing, setEditing] = useState<string | null>(null)   // row id, or 'new'
  const [draft, setDraft]     = useState<SharedContactInput>(EMPTY)
  const [saving, setSaving]   = useState(false)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const reload = useCallback(async () => {
    setLoading(true)
    try {
      const r = await contactsApi.listSharedBook()
      setRows(r.data.contacts)
      setError(null)
    } catch {
      setError("Le carnet partagé n'a pas pu être chargé.")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void reload() }, [reload])

  function startNew() {
    setDraft(EMPTY)
    setEditing('new')
  }

  function startEdit(c: SharedContact) {
    setDraft({
      display_name: c.display_name,
      organization: c.organization ?? '',
      job_title:    c.job_title ?? '',
      email:        c.email ?? '',
      phone:        c.phone ?? '',
      notes:        c.notes ?? '',
    })
    setEditing(c.id)
  }

  async function save() {
    if (!draft.display_name?.trim()) {
      setError('Le nom est obligatoire.')
      return
    }
    setSaving(true)
    try {
      if (editing === 'new') await contactsApi.createSharedContact(draft)
      else if (editing)      await contactsApi.updateSharedContact(editing, draft)
      setEditing(null)
      setError(null)
      await reload()
    } catch {
      setError("L'enregistrement a échoué.")
    } finally {
      setSaving(false)
    }
  }

  async function remove(c: SharedContact) {
    const ok = await confirm({
      title:       'Supprimer du carnet partagé',
      message:     `« ${c.display_name} » sera retiré du carnet partagé de l'instance. Les carnets personnels ne sont pas touchés.`,
      confirmLabel: 'Supprimer',
      variant:     'danger',
    })
    if (!ok) return
    try {
      await contactsApi.deleteSharedContact(c.id)
      await reload()
    } catch {
      setError('La suppression a échoué.')
    }
  }

  return (
    <div className="space-y-4">
      <Callout variant="info" title="Contacts sans compte">
        Ce carnet est tenu par l'administration et visible de tous les utilisateurs, en lecture
        seule : fournisseurs, partenaires, numéros d'urgence. Les comptes de l'instance n'y ont
        pas leur place — ils sont déjà dans l'annuaire. Le réglage ci-dessus décide seulement de
        sa diffusion ; les fiches restent modifiables ici même lorsqu'il est désactivé.
      </Callout>

      {error && <Callout variant="danger">{error}</Callout>}

      <div className="flex justify-end">
        <Button size="sm" onClick={startNew} disabled={editing === 'new'}>
          <Plus size={16} /> Ajouter une fiche
        </Button>
      </div>

      {editing && (
        <div className="rounded-xl border border-border bg-surface-1 p-4 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-text-primary">
              {editing === 'new' ? 'Nouvelle fiche' : 'Modifier la fiche'}
            </span>
            <Button variant="ghost" size="sm" onClick={() => { setEditing(null); setError(null) }}>
              <X size={16} />
            </Button>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Input label="Nom affiché" value={draft.display_name ?? ''}
                   onChange={e => setDraft({ ...draft, display_name: e.target.value })} />
            <Input label="Organisation" value={draft.organization ?? ''}
                   onChange={e => setDraft({ ...draft, organization: e.target.value })} />
            <Input label="Fonction" value={draft.job_title ?? ''}
                   onChange={e => setDraft({ ...draft, job_title: e.target.value })} />
            <Input label="Courriel" type="email" value={draft.email ?? ''}
                   onChange={e => setDraft({ ...draft, email: e.target.value })} />
            <Input label="Téléphone" value={draft.phone ?? ''}
                   onChange={e => setDraft({ ...draft, phone: e.target.value })} />
            <Input label="Note" value={draft.notes ?? ''}
                   onChange={e => setDraft({ ...draft, notes: e.target.value })} />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={() => { setEditing(null); setError(null) }}>
              Annuler
            </Button>
            <Button size="sm" onClick={() => void save()} disabled={saving}>
              {saving ? 'Enregistrement…' : 'Enregistrer'}
            </Button>
          </div>
        </div>
      )}

      <div className="rounded-xl border border-border overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-surface-2 text-text-secondary">
              <tr>
                <th className="text-left px-3 py-2">Nom</th>
                <th className="text-left px-3 py-2">Organisation</th>
                <th className="text-left px-3 py-2">Courriel</th>
                <th className="text-left px-3 py-2">Téléphone</th>
                <th className="px-3 py-2" />
              </tr>
            </thead>
            <tbody>
              {loading && (
                <tr><td colSpan={5} className="px-3 py-6 text-center text-text-secondary">Chargement…</td></tr>
              )}
              {!loading && rows.length === 0 && (
                <tr><td colSpan={5} className="px-3 py-6 text-center text-text-secondary">
                  Le carnet partagé est vide.
                </td></tr>
              )}
              {rows.map(c => (
                <tr key={c.id} className="border-t border-border">
                  <td className="px-3 py-2 text-text-primary">{c.display_name}</td>
                  <td className="px-3 py-2 text-text-secondary">{c.organization || '—'}</td>
                  <td className="px-3 py-2 text-text-secondary">{c.email || '—'}</td>
                  <td className="px-3 py-2 text-text-secondary">{c.phone || '—'}</td>
                  <td className="px-3 py-2">
                    <div className="flex justify-end gap-1">
                      <Button variant="ghost" size="sm" onClick={() => startEdit(c)} title="Modifier">
                        <Pencil size={15} />
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => void remove(c)} title="Supprimer">
                        <Trash2 size={15} />
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  )
}

/// Registers the contacts admin sections into the core console.
export function registerContactsAdmin() {
  ModuleAdminRegistry.register({
    moduleId:  'contacts',
    id:        'directory-notice',
    group:     'directory',
    position:  10,
    Component: DirectoryNotice,
  })
  ModuleAdminRegistry.register({
    moduleId:  'contacts',
    id:        'shared-book',
    group:     'shared-book',
    position:  20,
    Component: SharedBookManager,
  })
}
