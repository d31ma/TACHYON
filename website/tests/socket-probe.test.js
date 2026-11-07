import { describe, expect, test } from 'bun:test'
import { SocketProbe } from '../scripts/socket-probe.mjs'
import { FakeSocket } from './helpers/fake-socket.js'

describe('the WebSocket probe reads the first message', () => {
  test('a client that reports the close inside close() still returns the first message', async () => {
    FakeSocket.serverClose = null
    const first = await SocketProbe.firstMessage('ws://127.0.0.1/chat', /** @type {any} */ (FakeSocket))
    expect(first).toBe('{"text":"Welcome to the room"}')
  })

  test('a socket that closes before its first message reports the code, the stage and the reason', async () => {
    FakeSocket.serverClose = { code: 1011, reason: 'The socket handler stopped.' }
    const first = SocketProbe.firstMessage('ws://127.0.0.1/chat', /** @type {any} */ (FakeSocket))
    await expect(first).rejects.toThrow(
      'The socket closed with 1011 after it opened and before its first message: The socket handler stopped.',
    )
  })
})
