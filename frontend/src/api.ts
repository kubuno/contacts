import { api as apiClient } from '@kubuno/sdk'

export interface ContactField {
  label?: string
  value: string
  type: string
}

export interface AddressField {
  label?: string
  type: string
  street?: string
  city?: string
  region?: string
  postcode?: string
  country?: string
}

export interface DateField {
  label?: string
  type: string
  value: string
}

export interface CustomField {
  label: string
  value: string
}

export interface Contact {
  id: string
  owner_id: string
  given_name: string | null
  middle_name: string | null
  family_name: string | null
  name_prefix: string | null
  name_suffix: string | null
  nickname: string | null
  display_name: string
  organization: string | null
  department: string | null
  job_title: string | null
  avatar_path: string | null
  avatar_color: string
  emails: ContactField[]
  phones: ContactField[]
  addresses: AddressField[]
  urls: ContactField[]
  dates: DateField[]
  relations: ContactField[]
  instant_messages: ContactField[]
  custom_fields: CustomField[]
  notes: string | null
  pronouns: string | null
  is_starred: boolean
  is_trashed: boolean
  is_archived: boolean
  is_blocked: boolean
  last_interaction_at: string | null
  interaction_count: number
  kubuno_user_id: string | null
  vcard_uid: string
  etag: string
  import_source: string
  created_at: string
  updated_at: string
  /** Present on list responses (flattened) — ids of attached labels. */
  label_ids?: string[]
}

export interface Label {
  id: string
  owner_id: string
  name: string
  color: string
  icon: string | null
  is_system: boolean
  position: number
  contact_count: number
  created_at: string
  updated_at: string
}

export interface Reminder {
  id: string
  contact_id: string
  kind: string
  message: string | null
  remind_at: string
  recurrence: string
  is_done: boolean
  contact_name: string
  contact_avatar_color: string
}

export interface Interaction {
  id: string
  contact_id: string
  interaction_type: string
  summary: string | null
  source_module: string | null
  occurred_at: string
}

export interface ChangeEntry {
  field: string
  old_value: string | null
  new_value: string | null
  changed_at: string
}

export interface DuplicateGroup {
  reason: string
  contacts: Contact[]
}

export interface UpcomingDate {
  contact_id: string
  display_name: string
  avatar_color: string
  label: string
  date: string
  next_occurrence: string
  days_until: number
  age: number | null
}

export interface Share {
  id: string
  contact_id: string | null
  group_id: string | null
  token: string
  permission: string
  expires_at: string | null
  max_accesses: number | null
  access_count: number
  created_at: string
}

export interface ContactStats {
  total: number
  starred: number
  archived: number
  trashed: number
  blocked: number
  groups: number
  labels: number
  with_email: number
  with_phone: number
  with_avatar: number
  incomplete: number
  completeness_pct: number
}

export interface Group {
  id: string
  owner_id: string
  name: string
  color: string
  is_system: boolean
  contact_count: number
  created_at: string
  updated_at: string
}

export interface DirectoryProfile {
  kubuno_user_id: string
  display_name: string
  email: string
  avatar_url: string | null
  department: string | null
  job_title: string | null
  phone: string | null
}

export interface ListContactsParams {
  q?: string
  group_id?: string
  label_id?: string
  starred?: boolean
  trashed?: boolean
  archived?: boolean
  filter?: string
  sort?: string
  limit?: number
  offset?: number
}

const BASE = '/contacts'

export const contactsApi = {
  // ── Contacts ──────────────────────────────────────────────────────────────
  listContacts: (params: ListContactsParams = {}) =>
    apiClient.get<{ contacts: Contact[]; total: number }>(`${BASE}/contacts`, { params }),

  getContact: (id: string) =>
    apiClient.get<{ contact: Contact }>(`${BASE}/contacts/${id}`),

  createContact: (data: Partial<Contact>) =>
    apiClient.post<{ contact: Contact }>(`${BASE}/contacts`, data),

  updateContact: (id: string, data: Partial<Contact>) =>
    apiClient.patch<{ contact: Contact }>(`${BASE}/contacts/${id}`, data),

  trashContact: (id: string) =>
    apiClient.post(`${BASE}/contacts/${id}/trash`),

  restoreContact: (id: string) =>
    apiClient.post(`${BASE}/contacts/${id}/restore`),

  deleteContact: (id: string) =>
    apiClient.delete(`${BASE}/contacts/${id}/delete`),

  emptyTrash: () =>
    apiClient.delete(`${BASE}/contacts/trash`),

  starContact: (id: string) =>
    apiClient.post(`${BASE}/contacts/${id}/star`),

  unstarContact: (id: string) =>
    apiClient.post(`${BASE}/contacts/${id}/unstar`),

  findDuplicates: () =>
    apiClient.get<{ groups: DuplicateGroup[] }>(`${BASE}/contacts/duplicates`),

  mergeContacts: (primaryId: string, duplicateIds: string[]) =>
    apiClient.post<{ contact: Contact }>(`${BASE}/contacts/duplicates/merge`, {
      primary_id: primaryId, duplicate_ids: duplicateIds,
    }),

  ignoreDuplicate: (a: string, b: string) =>
    apiClient.post(`${BASE}/contacts/duplicates/ignore`, { contact_a: a, contact_b: b }),

  // ── Bulk + lifecycle ──────────────────────────────────────────────────────
  bulk: (ids: string[], action: string) =>
    apiClient.post<{ affected: number }>(`${BASE}/contacts/bulk`, { ids, action }),

  archiveContact:   (id: string) => apiClient.post(`${BASE}/contacts/${id}/archive`),
  unarchiveContact: (id: string) => apiClient.post(`${BASE}/contacts/${id}/unarchive`),
  blockContact:     (id: string) => apiClient.post(`${BASE}/contacts/${id}/block`),
  unblockContact:   (id: string) => apiClient.post(`${BASE}/contacts/${id}/unblock`),

  // ── History / interactions / birthdays / follow-up ────────────────────────
  getHistory: (id: string) =>
    apiClient.get<{ history: ChangeEntry[] }>(`${BASE}/contacts/${id}/history`),
  listInteractions: (id: string) =>
    apiClient.get<{ interactions: Interaction[] }>(`${BASE}/contacts/${id}/interactions`),
  addInteraction: (id: string, interaction_type: string, summary?: string) =>
    apiClient.post(`${BASE}/contacts/${id}/interactions`, { interaction_type, summary }),
  birthdays: (days = 365) =>
    apiClient.get<{ dates: UpcomingDate[] }>(`${BASE}/contacts/birthdays`, { params: { days } }),
  frequent: (limit = 12) =>
    apiClient.get<{ contacts: Contact[] }>(`${BASE}/contacts/frequent`, { params: { limit } }),
  recent: (limit = 12) =>
    apiClient.get<{ contacts: Contact[] }>(`${BASE}/contacts/recent`, { params: { limit } }),
  followUp: (days = 90, limit = 20) =>
    apiClient.get<{ contacts: Contact[] }>(`${BASE}/contacts/follow-up`, { params: { days, limit } }),

  // ── Labels ────────────────────────────────────────────────────────────────
  listLabels: () =>
    apiClient.get<{ labels: Label[] }>(`${BASE}/labels`),
  createLabel: (name: string, color?: string, icon?: string) =>
    apiClient.post<{ label: Label }>(`${BASE}/labels`, { name, color, icon }),
  updateLabel: (id: string, data: { name?: string; color?: string; icon?: string }) =>
    apiClient.patch<{ label: Label }>(`${BASE}/labels/${id}`, data),
  deleteLabel: (id: string) =>
    apiClient.delete(`${BASE}/labels/${id}`),
  addLabelMembers: (labelId: string, contactIds: string[]) =>
    apiClient.post(`${BASE}/labels/${labelId}/members`, { contact_ids: contactIds }),
  removeLabelMembers: (labelId: string, contactIds: string[]) =>
    apiClient.delete(`${BASE}/labels/${labelId}/members`, { data: { contact_ids: contactIds } }),

  // ── Reminders ─────────────────────────────────────────────────────────────
  listReminders: (includeDone = false) =>
    apiClient.get<{ reminders: Reminder[]; due_count: number }>(`${BASE}/reminders`, { params: { include_done: includeDone } }),
  createReminder: (data: { contact_id: string; kind?: string; message?: string; remind_at: string; recurrence?: string }) =>
    apiClient.post<{ reminder: Reminder }>(`${BASE}/reminders`, data),
  updateReminder: (id: string, data: { message?: string; remind_at?: string; recurrence?: string; is_done?: boolean }) =>
    apiClient.patch<{ reminder: Reminder }>(`${BASE}/reminders/${id}`, data),
  deleteReminder: (id: string) =>
    apiClient.delete(`${BASE}/reminders/${id}`),

  // ── Shares ────────────────────────────────────────────────────────────────
  listShares: () =>
    apiClient.get<{ shares: Share[] }>(`${BASE}/shares`),
  createShare: (data: { contact_id?: string; group_id?: string; expires_in_days?: number; max_accesses?: number; password?: string }) =>
    apiClient.post<{ share: Share }>(`${BASE}/shares`, data),
  revokeShare: (id: string) =>
    apiClient.delete(`${BASE}/shares/${id}`),

  // ── Settings / stats / CardDAV ────────────────────────────────────────────
  getSettings: () =>
    apiClient.get<{ settings: Record<string, unknown> }>(`${BASE}/settings`),
  updateSettings: (patch: Record<string, unknown>) =>
    apiClient.patch<{ settings: Record<string, unknown> }>(`${BASE}/settings`, patch),
  getStats: () =>
    apiClient.get<{ stats: ContactStats }>(`${BASE}/stats`),
  cardDavInfo: () =>
    apiClient.get<{ configured: boolean; username: string; path: string }>(`${BASE}/carddav/token`),
  cardDavGenerate: () =>
    apiClient.post<{ token: string; username: string; url: string }>(`${BASE}/carddav/token`),
  cardDavRevoke: () =>
    apiClient.delete(`${BASE}/carddav/token`),

  exportCsv: (params: { group_id?: string; starred?: boolean } = {}) => {
    const q = new URLSearchParams()
    if (params.group_id) q.set('group_id', params.group_id)
    if (params.starred !== undefined) q.set('starred', String(params.starred))
    window.open(`${BASE}/contacts/export.csv${q.toString() ? '?' + q.toString() : ''}`, '_blank')
  },

  uploadAvatar: (id: string, file: File) => {
    const fd = new FormData()
    fd.append('avatar', file)
    return apiClient.post<{ avatar_path: string }>(`${BASE}/contacts/${id}/avatar`, fd, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
  },

  avatarUrl: (id: string) => `${BASE}/contacts/${id}/avatar`,

  // ── Groups ────────────────────────────────────────────────────────────────
  listGroups: () =>
    apiClient.get<{ groups: Group[] }>(`${BASE}/groups`),

  createGroup: (name: string, color?: string) =>
    apiClient.post<{ group: Group }>(`${BASE}/groups`, { name, color }),

  updateGroup: (id: string, data: { name?: string; color?: string }) =>
    apiClient.patch<{ group: Group }>(`${BASE}/groups/${id}`, data),

  deleteGroup: (id: string) =>
    apiClient.delete(`${BASE}/groups/${id}`),

  addGroupMembers: (groupId: string, contactIds: string[]) =>
    apiClient.post(`${BASE}/groups/${groupId}/members`, { contact_ids: contactIds }),

  removeGroupMember: (groupId: string, contactId: string) =>
    apiClient.delete(`${BASE}/groups/${groupId}/members/${contactId}`),

  // ── Import / Export ───────────────────────────────────────────────────────
  exportVcf: (params: { group_id?: string; starred?: boolean } = {}) => {
    const q = new URLSearchParams()
    if (params.group_id) q.set('group_id', params.group_id)
    if (params.starred !== undefined) q.set('starred', String(params.starred))
    const url = `${BASE}/export.vcf${q.toString() ? '?' + q.toString() : ''}`
    window.open(url, '_blank')
  },

  importVcf: (file: File) => {
    const fd = new FormData()
    fd.append('file', file)
    return apiClient.post<{ total: number; imported: number; errors: number }>(
      `${BASE}/import/vcf`, fd, { headers: { 'Content-Type': 'multipart/form-data' } }
    )
  },

  importCsv: (file: File) => {
    const fd = new FormData()
    fd.append('file', file)
    return apiClient.post<{ imported: number; errors: number }>(
      `${BASE}/import/csv`, fd, { headers: { 'Content-Type': 'multipart/form-data' } }
    )
  },

  // ── Annuaire ──────────────────────────────────────────────────────────────
  searchDirectory: (q: string) =>
    apiClient.get<{ profiles: DirectoryProfile[] }>(`${BASE}/directory`, { params: { q } }),

  addFromDirectory: (kubunoUserId: string) =>
    apiClient.post<{ contact: Contact }>(`${BASE}/directory/${kubunoUserId}/add`),
}
