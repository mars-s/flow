import { describe, expect, test } from 'bun:test'
import {
  formatDocumentTitle,
  FLOW_DOCUMENT_TITLE,
} from './use-document-title'

describe('formatDocumentTitle', () => {
  test('uses the product title without a section', () => {
    expect(formatDocumentTitle()).toBe(FLOW_DOCUMENT_TITLE)
    expect(formatDocumentTitle('   ')).toBe(FLOW_DOCUMENT_TITLE)
  })

  test('identifies the current browser surface', () => {
    expect(formatDocumentTitle('New Task')).toBe('New Task — Flow Web')
    expect(formatDocumentTitle('  General  ')).toBe('General — Flow Web')
  })

  test('does not duplicate the product title', () => {
    expect(formatDocumentTitle(FLOW_DOCUMENT_TITLE)).toBe(FLOW_DOCUMENT_TITLE)
  })
})
