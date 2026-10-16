#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { chromium } from 'playwright'

class ClientTest {
  static clientSource = `@HTTPClient({"adapter":"api"})
class ProductsClient {
  @GetRequest("/api/products")
  async all(@RequestParam("mode") mode) {}
}`
}

class ApiTraffic {
  static slowStarted = false
  static slowAborted = false
  static clientSentCookie = false
}

function write(root, name, contents) {
  const file = path.join(root, name)
  mkdirSync(path.dirname(file), {recursive:true})
  writeFileSync(file, contents)
}

function writeProductFixtures(root) {
  for (const extension of ['js', 'ts']) {
    write(root, `client/remotes/Products${extension.toUpperCase()}Client.${extension}`, ClientTest.clientSource.replace('ProductsClient', `Products${extension.toUpperCase()}Client`))
    write(root, `client/pages/${extension}/tac.html`, `<products-${extension} />`)
    write(root, `client/components/products/${extension}/tac.html`, `<main><p id="result">{products${extension.toUpperCase()}.result}</p></main>`)
    write(root, `client/services/Products${extension.toUpperCase()}Service.${extension}`, `
@Service()
@NeedsAll()
export default class Products${extension.toUpperCase()}Service {
  ${extension === 'ts' ? `readonly products${extension.toUpperCase()}Client!: Products${extension.toUpperCase()}Client;` : `products${extension.toUpperCase()}Client;`}
  result = 'pending';
  @OnStart()
  async initialize() {
    this.#refuseRuntimeContext();
    this.result = JSON.stringify(await this.products${extension.toUpperCase()}Client.all('normal'));
  }
  #refuseRuntimeContext() {
    if ('tac' in this) throw new Error('Tac must not expose a runtime context');
  }
}`)
  }
}

function writeProviderFixtures(root) {
  write(root, 'client/helpers/CatalogPriceHelper.js', `@Helper()
class CatalogPriceHelper {
  _prices = { 1: 5 };
  price(id) { return this._prices[id] ?? 0; }
}`)
  write(root, 'client/helpers/PricingHelper.js', `@Helper()
@NeedsAll()
class PricingHelper {
  catalogPriceHelper;
  productsJSClient;
  async quote() {
    const [product] = await this.productsJSClient.all('normal');
    return product.name + '=' + this.catalogPriceHelper.price(product.id);
  }
}`)
  write(root, 'client/pages/providers/tac.html', '<providers-panel />')
  write(root, 'client/components/providers/panel/tac.html', '<main><p id="quote">{providersPanel.quote}</p></main>')
  write(root, 'client/services/ProvidersPanelService.js', `
@Service()
@Needs(PricingHelper, CatalogPriceHelper)
export default class ProvidersPanelService {
  static _NOT_INJECTED = 'providers were not injected'
  pricingHelper;
  catalogPriceHelper;
  quote = 'pending';
  @OnStart()
  async initialize() {
    this.#requirePricingHelper();
    this.#requireSharedHelper();
    this.quote = await this.pricingHelper.quote() + ' direct=' + this.catalogPriceHelper.price(1);
  }
  #requirePricingHelper() {
    if (!(this.pricingHelper instanceof PricingHelper)) throw new Error(ProvidersPanelService._NOT_INJECTED);
  }
  #requireSharedHelper() {
    if (this.pricingHelper.catalogPriceHelper !== this.catalogPriceHelper) throw new Error(ProvidersPanelService._NOT_INJECTED);
  }
}`)
}

function writePendingFixtures(root) {
  write(root, 'client/pages/tac.html', '<gate-page />')
  write(root, 'client/components/gate/page/tac.html', '<main><button id="hide" on:click="gatePage.show = false">Hide</button><logic :if="gatePage.show"><pending-request /></logic></main>')
  write(root, 'client/services/GatePageService.js', "@Service()\nexport default class GatePageService { show = true; }")
  write(root, 'client/components/pending/request/tac.html', '<p id="pending">{pendingRequest.state}</p>')
  write(root, 'client/remotes/PendingClient.js', ClientTest.clientSource.replace('ProductsClient', 'PendingClient'))
  write(root, 'client/services/PendingRequestService.js', `
@Service()
@Lifetime({"value":"prototype"})
@NeedsAll()
export default class PendingRequestService {
  pendingClient;
  state = 'Pending';
  @OnStart()
  async initialize() { await this.pendingClient.all('slow'); }
}`)
}

function writeRequiredFixtures(root) {
  write(root, 'client/pages/drawer/tac.html', '<drawer-panel :open="true" />')
  write(root, 'client/pages/drawer/missing/tac.html', '<drawer-panel :open="null" />')
  write(root, 'client/components/drawer/panel/tac.html', '<p id="drawer">{drawer.open} {drawer.label}</p>')
  write(root, 'client/services/DrawerService.js', `
@Service()
@NeedsAll()
export default class DrawerService {
  open;
  label = 'Menu';
}`)
}

function writeBooleanFixtures(root) {
  write(root, 'client/pages/toggles/tac.html', '<toggle-panel />')
  write(root, 'client/components/toggle/panel/tac.html', '<main><button id="open" on:click="togglePanel.open = true">Open</button><button id="close" on:click="togglePanel.open = false">Close</button><div id="backdrop" :hidden="!togglePanel.open">Backdrop</div><button id="save" :disabled="!togglePanel.open">Save</button></main>')
  write(root, 'client/services/TogglePanelService.js', "@Service()\nexport default class TogglePanelService { open = false; }")
}

function writeSetupFixtures(root) {
  write(root, 'client/manifest.json', '{"tac":{"duplication":{"check":false}}}\n')
  write(root, 'client/setups/SharedPageSetup.js', `@Setup
export default class SharedPageSetup {
  static configured = 0
  static services = 0
  static configure() {
    SharedPageSetup.configured += 1
    return {}
  }
}`)
  write(root, 'client/pages/setups/tac.html', '<setup-panel />')
  write(root, 'client/components/setup/panel/tac.html', '<main><button id="first-read" on:click="firstPage.read()">First</button><button id="second-read" on:click="secondPage.read()">Second</button><p id="first-page">{firstPage.configured}/{firstPage.services}</p><p id="second-page">{secondPage.configured}/{secondPage.services}</p></main>')
  for (const name of ['First', 'Second']) {
    write(root, `client/services/${name}PageService.js`, `
@Service()
export default class ${name}PageService {
  configured = 0;
  services = 0;
  @OnStart()
  async initialize() {
    SharedPageSetup.services += 1;
    await Behaviors.create(SharedPageSetup, {partition: '${name.toLowerCase()}-page', signal: new AbortController().signal});
    this.read();
  }
  read() {
    this.configured = SharedPageSetup.configured;
    this.services = SharedPageSetup.services;
  }
}`)
  }
}

function checkSetupModules(root) {
  for (const name of ['First', 'Second']) {
    const module = readFileSync(path.join(root, `dist/.tachyon/services/${name}PageService.js`), 'utf8')
    assert.ok(!module.includes('class SharedPageSetup'), `${name}PageService imports the Setup instead of carrying its own copy`)
    assert.match(module.split('\n')[0], /import \{ __tachyon_create_behaviors__, SharedPageSetup \} from '\.\.\/behavior-runtime\.js';/)
  }
  const runtime = readFileSync(path.join(root, 'dist/.tachyon/behavior-runtime.js'), 'utf8')
  assert.equal(runtime.split('export class SharedPageSetup').length, 2, 'the shared runtime module holds the one Setup definition')
}

function serveFixture(root) {
  return createServer((request, response) => {
    const url = new URL(request.url, 'http://localhost')
    if (url.pathname === '/api/products') {
      ApiTraffic.clientSentCookie ||= Boolean(request.headers.cookie)
      response.setHeader('content-type', 'application/json')
      if (url.searchParams.get('mode') === 'slow') {
        ApiTraffic.slowStarted = true
        response.on('close', () => { ApiTraffic.slowAborted = true })
      } else response.end(JSON.stringify([{id:1,name:'Products'}]))
      return
    }
    try {
      const relative = decodeURIComponent(url.pathname).replace(/^\/+/, '')
      assert.ok(!relative.split('/').includes('..'))
      const file = relative.includes('.') ? relative : path.join(relative, 'index.html')
      const mime = file.endsWith('.js') ? 'text/javascript' : file.endsWith('.json') ? 'application/json' : 'text/html'
      response.setHeader('content-type', mime)
      response.end(readFileSync(path.join(root, 'dist', file)))
    } catch { response.writeHead(404); response.end() }
  })
}

async function checkPages(browser, origin) {
  const page = await browser.newPage()
  await page.context().addCookies([{name:'private-session',value:'test-only',url:origin}])
  for (const extension of ['js', 'ts']) {
    await page.goto(`${origin}/${extension}`)
    await page.waitForFunction(() => document.querySelector('#result')?.textContent.includes('Products'))
    assert.deepEqual(JSON.parse(await page.locator('#result').textContent()), [{id:1,name:'Products'}])
    assert.equal(ApiTraffic.clientSentCookie, false, 'Managed clients do not implicitly forward browser cookies')
  }
  await page.goto(`${origin}/drawer`)
  await page.waitForFunction(() => document.querySelector('#drawer')?.textContent.includes('Menu'))
  assert.equal(await page.locator('#drawer').textContent(), 'true Menu', 'a NeedsAll service receives its required prop and keeps its initialized state')
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/drawer/missing`)
  await page.waitForFunction(() => document.documentElement.dataset.tachyonRenderError === 'render_failed')
  assert.equal(await page.locator('#drawer').count(), 0, 'a null required prop never creates the service')
  assert.ok(errors.some((message) => message.includes("Tac service 'drawer' needs 'open'")), errors.join('\n'))
  page.removeAllListeners('pageerror')
  await page.goto(`${origin}/toggles`)
  await page.waitForFunction(() => document.querySelector('#backdrop')?.hasAttribute('hidden'))
  assert.equal(await page.locator('#save').isDisabled(), true, 'a true boolean attribute is written')
  await page.locator('#open').click()
  await page.waitForFunction(() => !document.querySelector('#backdrop').hasAttribute('hidden'))
  assert.equal(await page.locator('#save').isDisabled(), false, 'a false boolean attribute is left out')
  await page.locator('#close').click()
  await page.waitForFunction(() => document.querySelector('#backdrop').hasAttribute('hidden'))
  await page.goto(`${origin}/providers`)
  await page.waitForFunction(() => document.querySelector('#quote')?.textContent.includes('='))
  assert.equal(await page.locator('#quote').textContent(), 'Products=5 direct=5', 'a service receives its Inject-bound component and bare repository, and the component its own client and repository')
  await page.goto(`${origin}/setups`)
  await page.waitForFunction(() => document.querySelector('#first-page')?.textContent !== '0/0' && document.querySelector('#second-page')?.textContent !== '0/0')
  await page.locator('#first-read').click()
  await page.locator('#second-read').click()
  const setupViews = async () => [await page.locator('#first-page').textContent(), await page.locator('#second-page').textContent()]
  let seen = await setupViews()
  for (let retry = 0; seen.join() !== '1/2,1/2' && retry < 100; retry++) {
    await page.waitForTimeout(20)
    seen = await setupViews()
  }
  assert.deepEqual(seen, ['1/2', '1/2'], 'two services on one page see one Setup class that configured once')
  await page.goto(origin)
  await page.waitForSelector('#pending')
  await assert.doesNotReject(async () => {
    for (let retry = 0; !ApiTraffic.slowStarted && retry < 100; retry++) await page.waitForTimeout(20)
    assert.equal(ApiTraffic.slowStarted, true)
  })
  await page.locator('#hide').click()
  await page.waitForSelector('#pending', {state:'detached'})
  for (let retry = 0; !ApiTraffic.slowAborted && retry < 100; retry++) await page.waitForTimeout(20)
  assert.equal(ApiTraffic.slowAborted, true, 'Removing a component aborts the managed client transport of its prototype service')
}

async function main() {
  const root = mkdtempSync(path.join(tmpdir(), 'tac-clients-'))
  const binary = process.env.TAC_BIN
  assert.ok(binary, 'Set TAC_BIN to the newly compiled ty binary')
  writeProductFixtures(root)
  writeProviderFixtures(root)
  writePendingFixtures(root)
  writeRequiredFixtures(root)
  writeBooleanFixtures(root)
  writeSetupFixtures(root)
  let browser
  let server
  try {
    const built = spawnSync(binary, ['build', root], {encoding:'utf8', timeout:120000,
      env:{...process.env,PATH:`${path.resolve('node_modules/.bin')}${path.delimiter}${process.env.PATH || ''}`}})
    assert.equal(built.status, 0, built.stdout + built.stderr)
    checkSetupModules(root)
    server = serveFixture(root)
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
    browser = await chromium.launch({headless:true})
    const origin = `http://127.0.0.1:${server.address().port}`
    await checkPages(browser, origin)
    console.log('Tac Client browser contract passed: JS, TS, annotation discovery, NeedsAll required props, boolean attributes, Inject-bound component and repository providers, one Setup for two services on a page, prototype service disposal.')
  } finally {
    await browser?.close()
    server?.closeAllConnections()
    if (server) await new Promise(resolve => server.close(resolve))
    rmSync(root, {recursive:true,force:true})
  }
}

await main()
