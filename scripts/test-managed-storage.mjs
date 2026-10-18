#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { mkdtemp, mkdir, writeFile, realpath, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { randomUUID } from 'node:crypto'
import { setTimeout as delay } from 'node:timers/promises'
import assert from 'node:assert/strict'
import { withServer } from './release/server-probe.mjs'

class ManagedStorageTest {
  static root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  static password = 'tachyon_ephemeral_fixture_only'
  static fixtures = [
    { driver: 'postgresql', image: 'postgres:18-alpine@sha256:d3e1620b530c944afa6e887d22eb899824da68e19c52024bf98f5220c88a65b2', port: 5432, username: 'postgres', environment: ['POSTGRES_PASSWORD', 'POSTGRES_DB=tachyon'] },
    { driver: 'mysql', image: 'mysql:8.4@sha256:85b9bf2e29cf836ecb8c2a15a935d4ba0c606631dff1dd79531a11983c638f2a', port: 3306, username: 'root', environment: ['MYSQL_ROOT_PASSWORD', 'MYSQL_DATABASE=tachyon'] },
    { driver: 'mariadb', image: 'mariadb:11.8@sha256:2d2f4095530294735a857cfe22bb101e19b0849b416911c796ec4aa81b164a62', port: 3306, username: 'root', environment: ['MARIADB_ROOT_PASSWORD', 'MARIADB_DATABASE=tachyon'] },
    { driver: 'redis', image: 'redis:8-alpine@sha256:becdda6c7f4b3fb42e42fd7f120bbf5c54c4caaaf16f26da24e4563d2c1f0576', port: 6379, environment: [] },
  ]
  static owned = []
  static scratchRoot = resolve(process.env.TAC_STORAGE_TEST_ROOT || tmpdir())
  static scratch = null
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
  const output = command('cargo', ['test', '-p', 'tachyon-core', '--lib', '--no-run', '--locked', '--message-format=json'], { timeout: 1_200_000 })
  const artifacts = output.split('\n').map(line => { try { return JSON.parse(line) } catch { return null } })
  const executable = artifacts.find(item => item?.reason === 'compiler-artifact' && item.target?.name === 'tachyon_core' && item.profile?.test && item.executable)?.executable
  if (!executable) throw new Error('Cargo did not report the core test executable.')
  return executable
}

async function buildCli() {
  if (process.env.TAC_BIN) return resolve(process.env.TAC_BIN)
  console.log('Building the CLI once for the FYLO server phase...')
  const output = command('cargo', ['build', '-p', 'tachyon-cli', '--locked', '--message-format=json'], { timeout: 1_200_000 })
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
  await write('server/application.json', JSON.stringify({ datasource: { driver: 'fylo', url: `fylo://${fyloRoot}` } }))
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
  await writeFile(path('server.ext'), 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n')
  command('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', path('ca.key'), '-out', path('ca.crt'), '-days', '1', '-subj', '/CN=Tachyon-Fixture-CA'])
  command('openssl', ['req', '-new', '-newkey', 'rsa:2048', '-nodes', '-keyout', path('server.key'), '-out', path('server.csr'), '-subj', '/CN=localhost'])
  command('openssl', ['x509', '-req', '-in', path('server.csr'), '-CA', path('ca.crt'), '-CAkey', path('ca.key'), '-CAcreateserial', '-out', path('server.crt'), '-days', '1', '-extfile', path('server.ext')])
  await writeFile(path('mysql.cnf'), '[mysqld]\nssl-ca=/tmp/tachyon-ca.crt\nssl-cert=/tmp/tachyon-server.crt\nssl-key=/tmp/tachyon-server.key\nrequire_secure_transport=ON\n')
}

function ping(fixture, id, tls) {
  if (fixture.driver === 'redis') return ['exec', id, 'redis-cli', 'ping']
  if (fixture.driver === 'postgresql') return ['exec', '--env', `PGPASSWORD=${ManagedStorageTest.password}`, id, 'psql', '--host=127.0.0.1', '--username=postgres', '-d', 'tachyon', '-tAc', 'SELECT 1']
  const client = fixture.driver === 'mysql' ? 'mysql' : 'mariadb'
  const flags = fixture.driver === 'mysql'
    ? [tls ? '--ssl-mode=VERIFY_IDENTITY' : '--ssl-mode=REQUIRED']
    : [tls ? '--ssl-verify-server-cert' : '--skip-ssl']
  if (tls) flags.push('--ssl-ca=/tmp/tachyon-ca.crt')
  return ['exec', '--env', `MYSQL_PWD=${ManagedStorageTest.password}`, id, client, '--user=root', '--host=127.0.0.1', '--database=tachyon', ...flags, '--batch', '--skip-column-names', '-e', 'SELECT 1']
}

async function ready(fixture, id, tls = false) {
  const deadline = Date.now() + 90_000
  let lastFailure = ''
  do {
    try { command('docker', ping(fixture, id, tls), { timeout: 3000 }); return } catch (error) {
      lastFailure = error.message
    }
    await delay(250)
  } while (Date.now() < deadline)
  throw new Error(`${fixture.driver} did not become ready within 90 seconds. ${lastFailure}`)
}

async function start(fixture) {
  console.log(`Starting isolated ${fixture.driver} fixture...`)
  const environment = fixture.environment.flatMap(value => ['--env', value.includes('=') ? value : `${value}=${ManagedStorageTest.password}`])
  const id = command('docker', ['run', '--detach', '--rm', '--name', `tachyon-storage-${randomUUID()}`, '--publish', `127.0.0.1::${fixture.port}`, ...environment, fixture.image], { timeout: 300_000 })
  if (!/^[a-f0-9]{64}$/.test(id)) throw new Error('Docker returned an invalid fixture identifier.')
  ManagedStorageTest.owned.push(id)
  await ready(fixture, id)
  if (fixture.driver !== 'redis') {
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
    command('docker', ['restart', id])
    await ready(fixture, id, true)
  }
  const address = command('docker', ['port', id, `${fixture.port}/tcp`])
  const match = /^127\.0\.0\.1:(\d+)$/.exec(address)
  if (!match) throw new Error('Fixture publication was not confined to IPv4 loopback.')
  return Number(match[1])
}

async function runDriverTest(binary, fixture) {
  const port = await start(fixture)
  const environment = { ...process.env }
  let test = 'storage::tests::sql_real_repository_nullable_columns'
  if (fixture.driver === 'redis') {
    environment.YON_TEST_REDIS_URL = `redis://127.0.0.1:${port}`
    test = 'storage::tests::redis_real_json_ttl_and_delete'
  } else {
    Object.assign(environment, {
      YON_TEST_DATASOURCE_DRIVER: fixture.driver,
      YON_TEST_DATASOURCE_URL: `${fixture.driver === 'mariadb' ? 'mysql' : fixture.driver}://127.0.0.1:${port}/tachyon`,
      YON_TEST_DATASOURCE_USERNAME: fixture.username,
      YON_TEST_DATASOURCE_PASSWORD: ManagedStorageTest.password,
      YON_TEST_DATASOURCE_CA: join(ManagedStorageTest.scratch, 'ca.crt'),
    })
  }
  console.log(command(binary, [test, '--exact', '--ignored'], { env: environment, timeout: 60_000 }))
}

async function main() {
  try {
    command('docker', ['info', '--format', '{{.ServerVersion}}'])
    const binary = await buildTests()
    console.log(command(binary, ['storage::tests', '--skip', 'storage::tests::redis_real_json_ttl_and_delete', '--skip', 'storage::tests::sql_real_repository_nullable_columns'],
      { timeout: 120_000, env: { ...process.env, TAC_STORAGE_REQUIRE_TOOLCHAINS: '1' } }))
    await fylo()
    await certificates()
    for (const fixture of ManagedStorageTest.fixtures) await runDriverTest(binary, fixture)
    console.log('PASS: SQLite, FYLO, PostgreSQL, MySQL, MariaDB and Redis real driver contracts. SQL Server is not qualified by this harness.')
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
