import { useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Upload, X } from 'lucide-react'
import { contactsApi } from './api'
import { useContactsStore } from './store'
import { Button } from '@ui'

interface Props {
  onClose: () => void
}

export default function ImportModal({ onClose }: Props) {
  const { t } = useTranslation('contacts')
  const [file, setFile] = useState<File | null>(null)
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<{ imported: number; errors: number } | null>(null)
  const [formatError, setFormatError] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const { fetchContacts } = useContactsStore()

  async function handleImport() {
    if (!file) return
    setLoading(true)
    try {
      const ext = file.name.split('.').pop()?.toLowerCase()
      let res
      if (ext === 'vcf') {
        res = await contactsApi.importVcf(file)
        setResult({ imported: res.data.imported, errors: res.data.errors })
      } else if (ext === 'csv') {
        res = await contactsApi.importCsv(file)
        setResult({ imported: res.data.imported, errors: res.data.errors })
      } else {
        setFormatError(true)
        return
      }
      await fetchContacts()
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-md">
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <h2 className="text-base font-semibold text-text-primary">{t('import_contacts')}</h2>
          <button onClick={onClose} className="text-text-secondary hover:text-text-primary"><X size={16} /></button>
        </div>

        <div className="p-5 space-y-4">
          {!result ? (
            <>
              <div
                className="border-2 border-dashed border-border rounded-xl p-8 text-center cursor-pointer hover:border-primary transition-colors"
                onClick={() => inputRef.current?.click()}
              >
                <Upload className="mx-auto mb-2 text-text-secondary" size={24} />
                <p className="text-sm text-text-secondary">
                  {file ? file.name : t('contacts_import_dropzone')}
                </p>
                <input
                  ref={inputRef}
                  type="file"
                  accept=".vcf,.csv"
                  className="hidden"
                  onChange={e => setFile(e.target.files?.[0] ?? null)}
                />
              </div>

              {formatError && (
                <p className="text-sm text-danger bg-danger/5 border border-danger/20 rounded-lg px-3 py-2">
                  {t('contacts_import_format_error')}
                </p>
              )}
              <Button
                className="w-full"
                onClick={() => { setFormatError(false); handleImport() }}
                disabled={!file}
                loading={loading}
              >
                {t('import')}
              </Button>
            </>
          ) : (
            <div className="text-center py-4">
              <p className="text-lg font-semibold text-text-primary">{t('contacts_import_done', { count: result.imported })}</p>
              {result.errors > 0 && (
                <p className="text-sm text-text-secondary mt-1">{t('contacts_import_errors', { count: result.errors })}</p>
              )}
              <Button className="mt-4" onClick={onClose}>
                {t('common_close')}
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
