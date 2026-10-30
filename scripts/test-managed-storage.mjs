#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import { connect, createServer } from 'node:net'
import { copyFile, mkdtemp, mkdir, readFile, writeFile, realpath, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { randomUUID } from 'node:crypto'
import { setTimeout as delay } from 'node:timers/promises'
import assert from 'node:assert/strict'
import { withServer } from './release/server-probe.mjs'

class ManagedStorageTest {
  static root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  static password = 'Tachyon_ephemeral_fixture_only_1'
  static fixtures = [
    { driver: 'postgresql', image: 'postgres:18-alpine@sha256:d3e1620b530c944afa6e887d22eb899824da68e19c52024bf98f5220c88a65b2', port: 5432, username: 'postgres', environment: ['POSTGRES_PASSWORD', 'POSTGRES_DB=tachyon'] },
    { driver: 'mysql', image: 'mysql:8.4@sha256:85b9bf2e29cf836ecb8c2a15a935d4ba0c606631dff1dd79531a11983c638f2a', port: 3306, username: 'root', environment: ['MYSQL_ROOT_PASSWORD', 'MYSQL_DATABASE=tachyon'] },
    { driver: 'mariadb', image: 'mariadb:11.8@sha256:2d2f4095530294735a857cfe22bb101e19b0849b416911c796ec4aa81b164a62', port: 3306, username: 'root', environment: ['MARIADB_ROOT_PASSWORD', 'MARIADB_DATABASE=tachyon'] },
    { driver: 'sqlserver', image: 'mcr.microsoft.com/mssql/server:2022-latest@sha256:cf8f0ab2ebe61feec2eed2629061a07d247f791c2a8ddda74a4ac9f06a291718', platform: 'linux/amd64', readySeconds: 240, port: 1433, username: 'sa', environment: ['ACCEPT_EULA=Y', 'MSSQL_SA_PASSWORD'] },
    { driver: 'redis', image: 'redis:8-alpine@sha256:becdda6c7f4b3fb42e42fd7f120bbf5c54c4caaaf16f26da24e4563d2c1f0576', port: 6379, environment: [] },
  ]
  static concurrentFyloPatches = 'storage::tests::fylo_documents::concurrent_fylo_patches_never_lose_an_update'
  static sqlcmd = '/opt/mssql-tools18/bin/sqlcmd'
  static sqlServerFolder = '/var/opt/mssql/tachyon'
  static owned = []
  static scratchRoot = resolve(process.env.TAC_STORAGE_TEST_ROOT || tmpdir())
  static scratch = null
  static withoutDocker = process.argv.includes('--without-docker')
  static drivers = process.argv.find(argument => argument.startsWith('--drivers='))?.slice('--drivers='.length).split(',').filter(Boolean) ?? []
  static serving = process.argv.includes('--serve')
  static remote = process.argv.find(argument => argument.startsWith('--remote='))?.slice('--remote='.length)
  static publishAddress = process.env.TAC_STORAGE_PUBLISH_ADDRESS || '127.0.0.1'
  static serveFolder = process.env.TAC_STORAGE_SERVE_DIR
  static serveMinutes = Number(process.env.TAC_STORAGE_SERVE_MINUTES || 60)
  static redisServers = [
    { name: 'redis-server', program: 'redis-server', args: port => ['--port', String(port), '--bind', '127.0.0.1', '--save', '', '--appendonly', 'no'] },
    { name: 'Garnet', program: process.env.TAC_GARNET_SERVER || 'garnet-server', args: port => ['--port', String(port), '--bind', '127.0.0.1'] },
  ]
}

await mkdir(ManagedStorageTest.scratchRoot, { recursive: true })
ManagedStorageTest.scratch = await mkdtemp(join(ManagedStorageTest.scratchRoot, 'tachyon-storage-'))

function command(program, args, options = {}) {
  const result = spawnSync(program, args, { cwd: ManagedStorageTest.root, encoding: 'utf8', timeout: 120_000, maxBuffer: 8 * 1024 * 1024, ...options })
  if (result.error || result.status !== 0) throw new Error(`${program} ${args[0] || ''} failed: ${result.error?.message || result.stderr || result.stdout}`)
  return result.stdout.trim()
}

async function buildTests() {
  if (process.env.TAC_STORAGE_TEST_BINARY) return resolve(process.env.TAC_STORAGE_TEST_BINARY)
  console.log('Building the core test executable once...')
  const output = command('cargo', ['test', '-p', 'tachyon-core', '--lib', '--features', 'storage-all', '--no-run', '--locked', '--message-format=json'], { timeout: 1_200_000 })
  const artifacts = output.split('\n').map(line => { try { return JSON.parse(line) } catch { return null } })
  const executable = artifacts.find(item => item?.reason === 'compiler-artifact' && item.target?.name === 'tachyon_core' && item.profile?.test && item.executable)?.executable
  if (!executable) throw new Error('Cargo did not report the core test executable.')
  return executable
}

async function buildCli() {
  if (process.env.TAC_BIN) return resolve(process.env.TAC_BIN)
  console.log('Building the CLI once for the FYLO server phase...')
  const output = command('cargo', ['build', '-p', 'tachyon-cli', '--features', 'storage-all', '--locked', '--message-format=json'], { timeout: 1_200_000 })
  const artifacts = output.split('\n').map(line => { try { return JSON.parse(line) } catch { return null } })
  const executable = artifacts.findLast(item => item?.reason === 'compiler-artifact' && item.target?.name === 'ty' && item.executable)?.executable
  if (!executable) throw new Error('Cargo did not report the ty executable.')
  return executable
}

function fyloExec(fyloRoot, request) {
  const answer = JSON.parse(command('fylo', ['exec', '--request', JSON.stringify(request), '--root', fyloRoot], { timeout: 30_000 }))
  if (answer.protocolVersion !== 1 || answer.ok !== true) throw new Error(`fylo ${request.op} failed: ${JSON.stringify(answer.error ?? answer)}`)
  return answer.result
}

async function writeFyloProject(project, fyloRoot) {
  const write = async (relative, contents) => {
    const file = join(project, relative)
    await mkdir(dirname(file), { recursive: true })
    await writeFile(file, contents)
  }
  await write('server/application.json', JSON.stringify({ datasource: { driver: 'fylo', url: `fylo:///${fyloRoot.replaceAll('\\', '/').replace(/^\/+/, '')}` } }))
  await write('server/controllers/ProductsController.js', `@RequestRoute("/products")
@Public
@NeedsAll()
class ProductsController {
productsService;
@GetRoute
get() { return this.productsService.all(); }
}
`)
  await write('server/models/ProductsListed.js', `/**
 * @typedef {Object} ProductsListed
 * @property {integer} count
 */
export {}
`)
  await write('server/services/ProductsService.js', `@Service()
@NeedsAll()
class ProductsService {
@Needs
dataSource;
@Needs
redisTemplate;
@Needs
applicationEventPublisher;
async all() {
  const products = await this.dataSource.query('SELECT * FROM products', []);
  /** @type {ProductsListed} */
  const listed = { count: products.length };
  this.applicationEventPublisher.publishEvent(listed, true);
  return products;
}
}
`)
}

async function fylo() {
  console.log('Qualifying the FYLO document driver...')
  command('fylo', ['version'], { timeout: 10_000 })
  const fyloRoot = await realpath(await mkdtemp(join(ManagedStorageTest.scratch, 'fylo-')))
  fyloExec(fyloRoot, { op: 'createCollection', collection: 'products' })
  const identifiers = [
    fyloExec(fyloRoot, { op: 'putData', collection: 'products', data: { name: 'anvil', qty: 2 } }),
    fyloExec(fyloRoot, { op: 'putData', collection: 'products', data: { name: 'rope', qty: 7 } }),
  ]
  const project = join(ManagedStorageTest.scratch, 'fylo-app')
  await writeFyloProject(project, fyloRoot)
  const cli = await buildCli()
  await withServer(cli, ['preview', project, '--host', '127.0.0.1', '--port', '0'], async request => {
    const response = await request('/products')
    assert.equal(response.status, 200, 'the FYLO-backed route answers')
    const products = await response.json()
    assert.deepEqual(
      products.map(product => product.name).sort(),
      ['anvil', 'rope'],
      'raw dataSource.query reaches FYLO through the autowired field',
    )
    assert.deepEqual(
      products.map(product => product.id).sort(),
      [...identifiers].sort(),
      'FYLO assigns the identifiers and the id field carries the TTID',
    )
  })
  console.log('FYLO driver qualified: canonical root, collection, documents, Yon autowired resources and raw SQL.')
}

async function certificates() {
  const path = name => join(ManagedStorageTest.scratch, name)
  const published = ManagedStorageTest.publishAddress === '127.0.0.1' ? '' : `,IP:${ManagedStorageTest.publishAddress}`
  await writeFile(path('server.ext'), `basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1${published}\n`)
  command('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', path('ca.key'), '-out', path('ca.crt'), '-days', '1', '-subj', '/CN=Tachyon-Fixture-CA'])
  command('openssl', ['req', '-new', '-newkey', 'rsa:2048', '-nodes', '-keyout', path('server.key'), '-out', path('server.csr'), '-subj', '/CN=localhost'])
  command('openssl', ['x509', '-req', '-in', path('server.csr'), '-CA', path('ca.crt'), '-CAkey', path('ca.key'), '-CAcreateserial', '-out', path('server.crt'), '-days', '1', '-extfile', path('server.ext')])
  await writeFile(path('mysql.cnf'), '[mysqld]\nssl-ca=/tmp/tachyon-ca.crt\nssl-cert=/tmp/tachyon-server.crt\nssl-key=/tmp/tachyon-server.key\nrequire_secure_transport=ON\n')
  const folder = ManagedStorageTest.sqlServerFolder
  await writeFile(path('mssql.conf'), `[network]\ntlscert = ${folder}/server.crt\ntlskey = ${folder}/server.key\ntlsprotocols = 1.2\nforceencryption = 1\n`)
}

function sqlServerQuery(id, query, tls = false) {
  return ['exec', '--env', `SQLCMDPASSWORD=${ManagedStorageTest.password}`, id, ManagedStorageTest.sqlcmd, '-S', '127.0.0.1', '-U', 'sa', '-C', '-b', ...(tls ? ['-N', '-d', 'tachyon'] : []), '-Q', query]
}

function installSqlServerTls(id) {
  const folder = ManagedStorageTest.sqlServerFolder
  command('docker', ['exec', '--user', 'root', id, 'mkdir', '-p', folder])
  for (const name of ['server.crt', 'server.key']) command('docker', ['cp', join(ManagedStorageTest.scratch, name), `${id}:${folder}/${name}`])
  command('docker', ['cp', join(ManagedStorageTest.scratch, 'mssql.conf'), `${id}:/var/opt/mssql/mssql.conf`])
  command('docker', ['exec', '--user', 'root', id, 'chown', '-R', 'mssql', folder, '/var/opt/mssql/mssql.conf'])
  command('docker', ['exec', '--user', 'root', id, 'chmod', '600', `${folder}/server.key`])
}

function installTls(fixture, id) {
  for (const name of ['ca.crt', 'server.crt', 'server.key']) command('docker', ['cp', join(ManagedStorageTest.scratch, name), `${id}:/tmp/tachyon-${name}`])
  const owner = fixture.driver === 'postgresql' ? 'postgres' : 'mysql'
  command('docker', ['exec', id, 'chown', `${owner}:${owner}`, '/tmp/tachyon-ca.crt', '/tmp/tachyon-server.crt', '/tmp/tachyon-server.key'])
  command('docker', ['exec', id, 'chmod', '600', '/tmp/tachyon-server.key'])
  if (fixture.driver === 'postgresql') {
    for (const setting of ["ssl = on", "ssl_cert_file = '/tmp/tachyon-server.crt'", "ssl_key_file = '/tmp/tachyon-server.key'"]) command('docker', ['exec', '--user', 'postgres', id, 'psql', '-c', `ALTER SYSTEM SET ${setting}`])
  } else {
    const location = fixture.driver === 'mysql' ? '/etc/mysql/conf.d/tachyon-tls.cnf' : '/etc/mysql/mariadb.conf.d/99-tachyon-tls.cnf'
    command('docker', ['cp', join(ManagedStorageTest.scratch, 'mysql.cnf'), `${id}:${location}`])
  }
}

function ping(fixture, id, tls) {
  if (fixture.driver === 'redis') return ['exec', id, 'redis-cli', 'ping']
  if (fixture.driver === 'sqlserver') return sqlServerQuery(id, 'SELECT 1', tls)
  if (fixture.driver === 'postgresql') return ['exec', '--env', `PGPASSWORD=${ManagedStorageTest.password}`, id, 'psql', '--host=127.0.0.1', '--username=postgres', '-d', 'tachyon', '-tAc', 'SELECT 1']
  const client = fixture.driver === 'mysql' ? 'mysql' : 'mariadb'
  const flags = fixture.driver === 'mysql'
    ? [tls ? '--ssl-mode=VERIFY_IDENTITY' : '--ssl-mode=REQUIRED']
    : [tls ? '--ssl-verify-server-cert' : '--skip-ssl']
  if (tls) flags.push('--ssl-ca=/tmp/tachyon-ca.crt')
  return ['exec', '--env', `MYSQL_PWD=${ManagedStorageTest.password}`, id, client, '--user=root', '--host=127.0.0.1', '--database=tachyon', ...flags, '--batch', '--skip-column-names', '-e', 'SELECT 1']
}

async function ready(fixture, id, tls = false) {
  const seconds = fixture.readySeconds ?? 90
  const deadline = Date.now() + seconds * 1000
  let lastFailure = ''
  do {
    try { command('docker', ping(fixture, id, tls), { timeout: 3000 }); return } catch (error) {
      lastFailure = error.message
    }
    await delay(250)
  } while (Date.now() < deadline)
  throw new Error(`${fixture.driver} did not become ready within ${seconds} seconds. ${lastFailure}`)
}

async function start(fixture) {
  console.log(`Starting isolated ${fixture.driver} fixture...`)
  const environment = fixture.environment.flatMap(value => ['--env', value.includes('=') ? value : `${value}=${ManagedStorageTest.password}`])
  const platform = fixture.platform ? ['--platform', fixture.platform] : []
  const address = ManagedStorageTest.publishAddress
  const id = command('docker', ['run', '--detach', '--rm', '--name', `tachyon-storage-${randomUUID()}`, ...platform, '--publish', `${address}::${fixture.port}`, ...environment, fixture.image], { timeout: 900_000 })
  if (!/^[a-f0-9]{64}$/.test(id)) throw new Error('Docker returned an invalid fixture identifier.')
  ManagedStorageTest.owned.push(id)
  await ready(fixture, id)
  if (fixture.driver === 'sqlserver') {
    command('docker', sqlServerQuery(id, 'CREATE DATABASE tachyon'))
    installSqlServerTls(id)
  } else if (fixture.driver !== 'redis') {
    installTls(fixture, id)
  }
  if (fixture.driver !== 'redis') {
    command('docker', ['restart', id])
    await ready(fixture, id, true)
  }
  const published = command('docker', ['port', id, `${fixture.port}/tcp`])
  const match = /^([\d.]+):(\d+)$/.exec(published)
  if (match?.[1] !== address) throw new Error(`Fixture publication was not confined to ${address}.`)
  return Number(match[2])
}

async function freePort() {
  const server = createServer()
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
  const { port } = server.address()
  await new Promise(resolve => server.close(resolve))
  return port
}

function pong(port) {
  return new Promise(resolve => {
    const socket = connect({ host: '127.0.0.1', port }, () => socket.write('PING\r\n'))
    socket.setTimeout(1000, () => socket.destroy())
    socket.once('data', data => { socket.destroy(); resolve(data.toString().startsWith('+PONG')) })
    socket.once('error', () => resolve(false))
    socket.once('close', () => resolve(false))
  })
}

async function localRedis(binary) {
  const local = ManagedStorageTest.redisServers.find(server => spawnSync(server.program, ['--version'], { timeout: 30_000 }).status === 0)
  if (!local) {
    console.log('Redis: no redis-server or garnet-server on this runner, so the Redis contract does not run here.')
    return false
  }
  const port = await freePort()
  const server = spawn(local.program, local.args(port), { stdio: 'ignore', windowsHide: true })
  try {
    const deadline = Date.now() + 30_000
    while (!(await pong(port))) {
      if (Date.now() > deadline) throw new Error(`The local ${local.name} did not answer PING within 30 seconds.`)
      await delay(250)
    }
    const environment = { ...process.env, YON_TEST_REDIS_URL: `redis://127.0.0.1:${port}` }
    console.log(command(binary, ['storage::tests::redis_real_json_ttl_and_delete', '--exact', '--ignored'], { env: environment, timeout: 60_000 }))
    console.log(`Redis contract passed against ${local.name}.`)
    return true
  } finally {
    server.kill()
  }
}

async function withoutDocker(binary) {
  await fylo()
  const redis = await localRedis(binary)
  console.log(`PASS: SQLite, FYLO${redis ? ' and Redis' : ''} real driver contracts without Docker.`)
}

/** The contract test and settings for one running fixture. */
function served(fixture, port) {
  const host = ManagedStorageTest.publishAddress
  if (fixture.driver === 'redis') return { driver: fixture.driver, test: 'storage::tests::redis_real_json_ttl_and_delete', settings: { YON_TEST_REDIS_URL: `redis://${host}:${port}` } }
  return {
    driver: fixture.driver,
    test: 'storage::tests::sql_real_repository_nullable_columns',
    settings: {
      YON_TEST_DATASOURCE_DRIVER: fixture.driver,
      YON_TEST_DATASOURCE_URL: `${fixture.driver === 'mariadb' ? 'mysql' : fixture.driver}://${host}:${port}/tachyon`,
      YON_TEST_DATASOURCE_USERNAME: fixture.username,
      YON_TEST_DATASOURCE_PASSWORD: ManagedStorageTest.password,
    },
    ca: fixture.driver === 'redis' ? undefined : 'ca.crt',
  }
}

function runServed(binary, entry, folder) {
  const environment = { ...process.env, ...entry.settings, ...(entry.ca ? { YON_TEST_DATASOURCE_CA: join(folder, entry.ca) } : {}) }
  console.log(command(binary, [entry.test, '--exact', '--ignored'], { env: environment, timeout: 120_000 }))
}

async function runDriverTest(binary, fixture) {
  runServed(binary, served(fixture, await start(fixture)), ManagedStorageTest.scratch)
}

/** Starts the chosen fixtures on TAC_STORAGE_PUBLISH_ADDRESS and keeps them until SIGINT, SIGTERM or TAC_STORAGE_SERVE_MINUTES, so another machine can run the contracts with --remote. */
async function serve(chosen) {
  await certificates()
  const folder = resolve(ManagedStorageTest.serveFolder || ManagedStorageTest.scratch)
  await mkdir(folder, { recursive: true })
  const fixtures = []
  for (const fixture of chosen) fixtures.push(served(fixture, await start(fixture)))
  await copyFile(join(ManagedStorageTest.scratch, 'ca.crt'), join(folder, 'ca.crt'))
  const file = join(folder, 'tachyon-storage-fixtures.json')
  await writeFile(file, `${JSON.stringify({ host: ManagedStorageTest.publishAddress, fixtures }, null, 2)}\n`)
  console.log(`SERVING ${fixtures.map(entry => entry.driver).join(', ')} on ${ManagedStorageTest.publishAddress}: copy ${file} and ca.crt to the other machine and run scripts/test-managed-storage.mjs --remote=<that json>. Send SIGINT or SIGTERM to stop; it stops by itself after ${ManagedStorageTest.serveMinutes} minutes.`)
  await new Promise(done => {
    const timer = setTimeout(done, ManagedStorageTest.serveMinutes * 60_000)
    const stop = () => { clearTimeout(timer); done() }
    process.once('SIGINT', stop)
    process.once('SIGTERM', stop)
  })
}

/** Runs the contracts against fixtures another machine serves (see --serve); no Docker here. */
async function remote(file) {
  const answer = JSON.parse(await readFile(file, 'utf8'))
  const fixtures = answer.fixtures.filter(entry => !ManagedStorageTest.drivers.length || ManagedStorageTest.drivers.includes(entry.driver))
  assert.ok(fixtures.length, `${file} serves none of ${ManagedStorageTest.drivers.join(', ')}`)
  const binary = await buildTests()
  for (const entry of fixtures) runServed(binary, entry, dirname(resolve(file)))
  console.log(`PASS: ${fixtures.map(entry => entry.driver).join(', ')} real driver contracts against the fixtures served from ${answer.host}.`)
}

function chosenFixtures() {
  const chosen = ManagedStorageTest.fixtures.filter(fixture => !ManagedStorageTest.drivers.length || ManagedStorageTest.drivers.includes(fixture.driver))
  assert.equal(chosen.length, ManagedStorageTest.drivers.length || ManagedStorageTest.fixtures.length, `--drivers takes ${ManagedStorageTest.fixtures.map(fixture => fixture.driver).join(', ')}`)
  return chosen
}

async function main() {
  try {
    if (ManagedStorageTest.remote) return await remote(ManagedStorageTest.remote)
    if (!ManagedStorageTest.withoutDocker) command('docker', ['info', '--format', '{{.ServerVersion}}'])
    if (ManagedStorageTest.serving) return await serve(chosenFixtures())
    const binary = await buildTests()
    const environment = { ...process.env, TAC_STORAGE_REQUIRE_TOOLCHAINS: '1' }
    console.log(command(binary, ['storage::tests', '--skip', 'storage::tests::redis_real_json_ttl_and_delete', '--skip', 'storage::tests::sql_real_repository_nullable_columns', '--skip', ManagedStorageTest.concurrentFyloPatches],
      { timeout: 300_000, env: environment }))
    console.log(command(binary, [ManagedStorageTest.concurrentFyloPatches, '--exact'], { timeout: 60_000, env: environment }))
    if (ManagedStorageTest.withoutDocker) return await withoutDocker(binary)
    await fylo()
    await certificates()
    const chosen = chosenFixtures()
    for (const fixture of chosen) await runDriverTest(binary, fixture)
    console.log(`PASS: SQLite, FYLO and ${chosen.map(fixture => fixture.driver).join(', ')} real driver contracts.`)
  } catch (error) {
    console.error(error.message)
    process.exitCode = 1
  } finally {
    for (const id of ManagedStorageTest.owned.reverse()) {
      try { command('docker', ['rm', '--force', '--volumes', id]) } catch (error) { console.error(`Fixture cleanup failed for ${id}: ${error.message}`); process.exitCode = 1 }
    }
    await rm(ManagedStorageTest.scratch, { recursive: true, force: true })
  }
}

await main()
