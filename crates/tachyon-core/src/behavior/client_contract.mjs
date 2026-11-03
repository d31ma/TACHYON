import assert from 'node:assert/strict'
import http from 'node:http'
import fs from 'node:fs/promises'
import { stripTypeScriptTypes } from 'node:module'

function respond(request, response) {
  ContractServer.received++
  const url = new URL(request.url, 'http://localhost')
  const mode = url.searchParams.get('mode')
  response.setHeader('content-type', 'application/json')
  if (mode === 'text') {
    response.setHeader('content-type', 'text/html'); response.end('<p>not JSON</p>'); return
  }
  if (mode === 'bad-json') { response.end('{'); return }
  if (mode === 'empty') { response.writeHead(204); response.end(); return }
  if (mode === 'redirect') {
    response.writeHead(302, { location: '/elsewhere' }); response.end(); return
  }
  if (mode === 'huge') { response.end(JSON.stringify('x'.repeat(1048577))); return }
  if (mode === 'slow') {
    ContractServer.notifySlow()
    const timer = setTimeout(() => response.end('{}'), 30000)
    response.on('close', () => clearTimeout(timer)); return
  }
  response.end(JSON.stringify({ query: mode, method: request.method, authorization: request.headers.authorization ?? null }))
}

class ContractServer {
  static received = 0
  static notifySlow
  static slowStarted = new Promise(resolve => { ContractServer.notifySlow = resolve })
  static server = http.createServer(respond)
}

await new Promise(resolve => ContractServer.server.listen(0, '127.0.0.1', resolve))
try {
  const origin = `http://127.0.0.1:${ContractServer.server.address().port}`
  globalThis.location = new URL(origin)
  let source = await fs.readFile(process.argv[2], 'utf8')
  if (process.argv[3] === 'ts') source = stripTypeScriptTypes(source)
  const module = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`)
  const first = new AbortController(), second = new AbortController()
  const left = await module.__tachyon_create_owner__({}, first.signal)
  const right = await module.__tachyon_create_owner__({}, second.signal)
  assert.notEqual(left.productsClient, right.productsClient)
  assert.equal(Object.keys(left).includes('productsClient'), false)
  assert.deepEqual(await left.productsClient.all('normal'), {query:'normal',method:'GET',authorization:null})
  await assert.rejects(left.productsClient.all('redirect'))
  await assert.rejects(left.productsClient.all('huge'))
  await assert.rejects(left.productsClient.all('text'))
  await assert.rejects(left.productsClient.all('bad-json'))
  assert.equal(await left.productsClient.all('empty'), null)
  const pending = left.productsClient.all('slow')
  const rejected = assert.rejects(pending)
  await ContractServer.slowStarted
  first.abort()
  await rejected
  const before = ContractServer.received
  await assert.rejects(left.productsClient.all('normal'))
  assert.equal(ContractServer.received, before)
  assert.equal((await right.productsClient.all('still-live')).query, 'still-live')
  second.abort()
  console.log('managed-client-contract:passed')
} finally {
  ContractServer.server.closeAllConnections()
  await new Promise(resolve => ContractServer.server.close(resolve))
}
