import { useEffect } from 'react'

export const FLOW_DOCUMENT_TITLE = 'Flow Web'

export function formatDocumentTitle(section?: string | null): string {
  const normalized = section?.trim()
  if (!normalized || normalized === FLOW_DOCUMENT_TITLE) return FLOW_DOCUMENT_TITLE
  return `${normalized} — ${FLOW_DOCUMENT_TITLE}`
}

export function useDocumentTitle(section?: string | null) {
  const title = formatDocumentTitle(section)
  useEffect(() => {
    document.title = title
  }, [title])
}
