import { describe, expect, test } from 'bun:test'
import type { FlowClient } from '@flow/client'
import { importDaemonPathAttachment } from './attachments'

describe('importDaemonPathAttachment', () => {
  test('asks the daemon to import an absolute path without sending file bytes', async () => {
    let command: unknown
    const client = {
      request: async (next: unknown) => {
        command = next
        return {
          type: 'attachmentStored',
          attachment: {
            reference: 'flow-attachment:one',
            path: '/home/me/.flow/attachments/one/logo.png',
            name: 'logo.png',
            isDir: false,
          },
        }
      },
    } as unknown as FlowClient

    const attachment = await importDaemonPathAttachment(client, '/Users/me/Pictures/logo.png')

    expect(command).toEqual({
      type: 'importPathAttachment',
      path: '/Users/me/Pictures/logo.png',
    })
    expect(attachment).toEqual({
      path: '/home/me/.flow/attachments/one/logo.png',
      mention: '/Users/me/Pictures/logo.png',
      name: 'logo.png',
      is_dir: false,
      is_image: true,
      blob_reference: 'flow-attachment:one',
    })
  })
})
