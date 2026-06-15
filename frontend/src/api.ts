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
  is_starred: boolean
  is_trashed: boolean
  kubuno_user_id: string | null
  vcard_uid: string
  etag: string
  import_source: string
  created_at: string
  updated_at: string
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
  starred?: boolean
  trashed?: boolean
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
    apiClient.get<{ groups: Contact[][] }>(`${BASE}/contacts/duplicates`),

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
