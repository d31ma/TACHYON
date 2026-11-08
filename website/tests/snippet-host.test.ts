import { afterEach, describe, expect, test } from 'bun:test'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { RunnerSettings } from '../runner/RunnerSettings.ts'
import { SnippetHost } from '../runner/SnippetHost.ts'

class HostFixture {
  static readonly ECHO = '75236e5355efdd11'
  static readonly ITEMS = 'e89aaa411a01f3b5'
  static readonly UNSTAGED = '0123456789abcdef'
  static readonly CLIENT = '203.0.113.7'
  static folders: string[] = []
  static hosts: SnippetHost[] = []

  static folder(): string {
    const folder = mkdtempSync(path.join(tmpdir(), 'snippet-host-'))
    HostFixture.folders.push(folder)
    return folder
  }

  static stage(projects: string, runId: string, routes: { method: string, path: string }[]): void {
    mkdirSync(path.join(projects, runId, 'project', 'server'), { recursive: true })
    writeFileSync(path.join(projects, runId, 'runnable.json'), JSON.stringify({ schema: 1, runId, source: `example/${runId}`, language: 'py', routes }))
  }

  static host(change: (settings: RunnerSettings) => void = () => {}): SnippetHost {
    const settings = new RunnerSettings()
    settings.port = 0
    settings.projectsFolder = HostFixture.folder()
    settings.temporaryFolder = HostFixture.folder()
    settings.tyCommand = [process.execPath, path.join(import.meta.dir, 'helpers', 'FakeTy.ts')]
    settings.childPath = path.dirname(process.execPath)
    settings.runMilliseconds = 1000
    settings.maxAnswerBytes = 1024
    HostFixture.stage(settings.projectsFolder, HostFixture.ECHO, [{ method: 'POST', path: '/echo' }, { method: 'GET', path: '/slow' }, { method: 'GET', path: '/big' }])
    HostFixture.stage(settings.projectsFolder, HostFixture.ITEMS, [{ method: 'GET', path: '/items/{id}' }])
    change(settings)
    const host = new SnippetHost(settings)
    HostFixture.hosts.push(host)
    return host
  }

  static run(host: SnippetHost, runId: string, body: object, headers: Record<string, string> = { 'x-snippet-client': HostFixture.CLIENT }): Promise<Response> {
    return fetch(`${host.url()}/runs/${runId}`, { method: 'POST', headers, body: JSON.stringify(body) })
  }

  static async answerOf(response: Response): Promise<Record<string, any>> {
    const result = await response.json() as Record<string, any>
    return JSON.parse(result.response.body)
  }
}

afterEach(async () => {
  await Promise.all(HostFixture.hosts.splice(0).map(host => host.stop()))
  for (const folder of HostFixture.folders.splice(0)) rmSync(folder, { recursive: true, force: true })
})

describe('the snippet host', () => {
  test('it runs an example route and forwards the request with a localhost Host header', async () => {
    const host = HostFixture.host()
    const response = await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo', query: { page: '2' }, body: '{"name":"ada"}' })
    expect(response.status).toBe(200)
    const answer = await HostFixture.answerOf(response)
    expect(answer).toMatchObject({ method: 'POST', path: '/echo', query: { page: '2' }, host: 'localhost', body: '{"name":"ada"}' })
  })

  test('the result reports the outcome, time and answer', async () => {
    const host = HostFixture.host()
    const result = await (await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })).json() as Record<string, any>
    expect(result.outcome).toBe('ok')
    expect(typeof result.durationMs).toBe('number')
    expect(result.response.status).toBe(200)
    expect(result.response.contentType).toContain('application/json')
    expect(result.response.truncated).toBe(false)
  })

  test('a route template accepts a path value', async () => {
    const host = HostFixture.host()
    const response = await HostFixture.run(host, HostFixture.ITEMS, { method: 'GET', path: '/items/42' })
    expect((await HostFixture.answerOf(response)).path).toBe('/items/42')
  })

  test('the example server gets a cleared environment with process isolation', async () => {
    const host = HostFixture.host()
    const answer = await HostFixture.answerOf(await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }))
    expect(answer.environment).toEqual(['HOME', 'LANG', 'PATH', 'TAC_CACHE_DIR', 'TMPDIR', 'YON_ISOLATION'])
  })

  test('the example runs from its own copy in the temporary folder', async () => {
    const host = HostFixture.host()
    const answer = await HostFixture.answerOf(await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }))
    expect(answer.project).not.toContain(HostFixture.ECHO)
    expect(path.basename(answer.project)).toBe('project')
  })

  test('two runs of one example reuse its server', async () => {
    const host = HostFixture.host()
    const first = await HostFixture.answerOf(await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }))
    const second = await HostFixture.answerOf(await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }))
    expect(second.pid).toBe(first.pid)
    expect(host.runningServers()).toBe(1)
  })

  test('an example that is not staged is refused without starting anything', async () => {
    const host = HostFixture.host()
    const response = await HostFixture.run(host, HostFixture.UNSTAGED, { method: 'GET', path: '/' })
    expect(response.status).toBe(404)
    expect(host.runningServers()).toBe(0)
  })

  test('a run id that is not 16 hex digits is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, '..%2Fetc', { method: 'GET', path: '/' })).status).toBe(404)
  })

  test('a path that is not a route of the example is refused', async () => {
    const host = HostFixture.host()
    const response = await HostFixture.run(host, HostFixture.ECHO, { method: 'GET', path: '/echo' })
    expect(response.status).toBe(404)
    expect((await response.json() as Record<string, any>).code).toBe('unknown-route')
  })

  test('a path that climbs out of the route is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, HostFixture.ITEMS, { method: 'GET', path: '/items/../admin' })).status).toBe(400)
  })

  test('an unknown method is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'TRACE', path: '/echo' })).status).toBe(400)
  })

  test('a body that is not text is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo', body: { name: 'ada' } })).status).toBe(400)
  })

  test('a request that is not JSON is refused', async () => {
    const host = HostFixture.host()
    const response = await fetch(`${host.url()}/runs/${HostFixture.ECHO}`, { method: 'POST', headers: { 'x-snippet-client': HostFixture.CLIENT }, body: 'name=ada' })
    expect(response.status).toBe(400)
  })

  test('a body larger than the limit is refused with 413', async () => {
    const host = HostFixture.host(settings => { settings.maxBodyBytes = 32 })
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo', body: 'x'.repeat(33) })).status).toBe(413)
  })

  test('too many query values are refused', async () => {
    const host = HostFixture.host()
    const query = Object.fromEntries(Array.from({ length: 17 }, (_, index) => [`q${index}`, 'v']))
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo', query })).status).toBe(400)
  })

  test('a run without the client header is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, {})).status).toBe(400)
  })

  test('a client header that is not an address is refused', async () => {
    const host = HostFixture.host()
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, { 'x-snippet-client': 'ada; drop' })).status).toBe(400)
  })

  test('the rate limit answers 429 with Retry-After', async () => {
    const host = HostFixture.host(settings => { settings.runsPerMinute = 1 })
    await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    const refused = await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    expect(refused.status).toBe(429)
    expect(Number(refused.headers.get('retry-after'))).toBeGreaterThan(0)
  })

  test('spoofed forwarding headers do not change the rate limit bucket', async () => {
    const host = HostFixture.host(settings => { settings.runsPerMinute = 1 })
    const spoofed = (address: string) => ({ 'x-snippet-client': HostFixture.CLIENT, 'x-forwarded-for': address, forwarded: `for=${address}`, 'x-real-ip': address })
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, spoofed('198.51.100.1'))).status).toBe(200)
    expect((await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, spoofed('198.51.100.2'))).status).toBe(429)
  })

  test('forwarding headers do not reach the example', async () => {
    const host = HostFixture.host()
    const headers = { 'x-snippet-client': HostFixture.CLIENT, 'x-forwarded-for': '198.51.100.1' }
    const answer = await HostFixture.answerOf(await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, headers))
    expect(answer.forwardedFor).toBeNull()
  })

  test('each client address has its own bucket', async () => {
    const host = HostFixture.host(settings => { settings.runsPerMinute = 1 })
    await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    const other = await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' }, { 'x-snippet-client': '203.0.113.8' })
    expect(other.status).toBe(200)
  })

  test('a slow example ends with outcome timeout', async () => {
    const host = HostFixture.host()
    const result = await (await HostFixture.run(host, HostFixture.ECHO, { method: 'GET', path: '/slow' })).json() as Record<string, any>
    expect(result.outcome).toBe('timeout')
    expect(result.response).toBeUndefined()
  })

  test('a large answer is cut at the limit and marked truncated', async () => {
    const host = HostFixture.host()
    const result = await (await HostFixture.run(host, HostFixture.ECHO, { method: 'GET', path: '/big' })).json() as Record<string, any>
    expect(result.response.body.length).toBe(1024)
    expect(result.response.truncated).toBe(true)
  })

  test('the busy answer is 503 when no server can be started', async () => {
    const host = HostFixture.host(settings => { settings.maxServers = 0 })
    const response = await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    expect(response.status).toBe(503)
    expect(response.headers.get('retry-after')).toBe('5')
  })

  test('at the server limit the least recently used idle server is stopped', async () => {
    const host = HostFixture.host(settings => { settings.maxServers = 1 })
    await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    await HostFixture.run(host, HostFixture.ITEMS, { method: 'GET', path: '/items/7' })
    expect(host.runningServers()).toBe(1)
  })

  test('an idle server is stopped and its temporary folder deleted', async () => {
    let temporary = ''
    const host = HostFixture.host(settings => { settings.idleMilliseconds = 200; temporary = settings.temporaryFolder })
    await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    expect(readdirSync(temporary).length).toBe(1)
    await Bun.sleep(600)
    expect(host.runningServers()).toBe(0)
    expect(readdirSync(temporary)).toEqual([])
  })

  test('stopping the host stops every server and deletes their folders', async () => {
    let temporary = ''
    const host = HostFixture.host(settings => { temporary = settings.temporaryFolder })
    await HostFixture.run(host, HostFixture.ECHO, { method: 'POST', path: '/echo' })
    await HostFixture.hosts.splice(0)[0].stop()
    expect(existsSync(temporary) ? readdirSync(temporary) : []).toEqual([])
  })

  test('the host listens on loopback only', () => {
    const host = HostFixture.host()
    expect(new URL(host.url()).hostname).toBe('127.0.0.1')
  })

  test('health reports how many examples can run', async () => {
    const host = HostFixture.host()
    expect(await (await fetch(`${host.url()}/health`)).json()).toEqual({ examples: 2 })
  })

  test('any other path is not found', async () => {
    const host = HostFixture.host()
    expect((await fetch(`${host.url()}/runs`)).status).toBe(404)
  })
})
