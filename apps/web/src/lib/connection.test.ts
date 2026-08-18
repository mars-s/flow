import { describe, expect, test } from 'bun:test'
import { normalizeDaemonAddress, validateConnectionConfig } from './connection'

describe('normalizeDaemonAddress', () => {
  test('normalizes host, HTTP, and daemon paths', () => {
    expect(normalizeDaemonAddress('host.example:34123')).toBe(
      'ws://host.example:34123',
    )
    expect(normalizeDaemonAddress('https://flow.example/v1?token=nope')).toBe(
      'wss://flow.example',
    )
    expect(normalizeDaemonAddress('HTTP://FLOW.EXAMPLE/v1')).toBe(
      'ws://flow.example',
    )
  })

  test('rejects unsupported schemes and credentials', () => {
    expect(() => normalizeDaemonAddress('ftp://flow.example')).toThrow()
    expect(() => normalizeDaemonAddress('ws://token@flow.example')).toThrow()
  })

  test('requires a token without putting it in the address', () => {
    expect(() =>
      validateConnectionConfig({ address: 'flow.example', token: '  ' }),
    ).toThrow('token')
    expect(
      validateConnectionConfig({ address: 'flow.example', token: 'secret' }),
    ).toEqual({
      address: 'ws://flow.example',
      token: 'secret',
      remember: false,
    })
  })
})
