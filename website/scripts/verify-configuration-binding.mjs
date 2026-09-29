import assert from 'node:assert/strict'
import { chromium } from 'playwright'
import { withServer } from '../../scripts/release/server-probe.mjs'

/**
 * @param {string} binary
 * @param {string} root
 */
export async function checkConfigurationBrowser(binary, root) {
  await withServer(binary, ['start', root, '--host', '127.0.0.1', '--port', '0'], async request => {
    const response = await request('/')
    assert.equal(response.status, 200)
    const browser = await chromium.launch({ timeout: 30_000 })
    try {
      const page = await browser.newPage()
      /** @type {string[]} */
      const errors = []
      page.on('pageerror', error => errors.push(error.message))
      page.on('console', message => {
        if (message.type() === 'error') errors.push(message.text())
      })
      await page.goto(response.url)
      await page.waitForFunction(() => document.querySelector('h1')?.textContent === 'Catalog').catch(error => {
        throw new Error(`${error.message}\nBrowser errors: ${errors.join('\n')}`)
      })
      await page.waitForFunction(() => document.querySelector('main p')?.textContent === 'Page size: 20')
      await page.waitForLoadState('networkidle')
      const markers = await page.locator('*').evaluateAll(elements => elements.flatMap(element =>
        [...element.attributes].filter(attribute => /^data-tachyon-.*error$/.test(attribute.name)).map(attribute => attribute.name)))
      assert.deepEqual(markers, [], 'The published configuration must initialize without runtime failure markers')
      assert.deepEqual(errors, [], 'The published configuration must initialize without browser errors')
    } finally {
      await browser.close()
    }
  })
}

/**
 * @param {string} extension
 * @param {string} service
 */
export function configurationTestFiles(extension, service) {
  if (extension === '.rs') return rustConfigurationTests(service)
  if (extension === '.cs') return csharpConfigurationTests(service)
  const swift = extension === '.swift'
  const configuration = swift
    ? '@TestSetup\nfinal class BindingTestSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }\n}\n'
    : '@TestSetup\nclass BindingTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n'
  const test = swift
    ? '@Test(BindingTestSetup, {"partition":"tests"})\nfinal class BindingServiceTest {\n    func testBindsPublishedConfiguration() async throws {\n        let settings = SettingsService()\n        try Assert.equals("Catalog", settings.title)\n        try Assert.equals(20, settings.pageSize)\n    }\n}\n'
    : '@Test(BindingTestSetup::class, {"partition":"tests"})\nclass BindingServiceTest {\n    fun testBindsPublishedConfiguration() {\n        val settings = SettingsService()\n        check(settings.title == "Catalog" && settings.pageSize == 20L)\n    }\n}\n'
  return [
    { name: `client/tests/BindingTestSetup${extension}`, code: configuration },
    { name: `client/tests/BindingServiceTest${extension}`, code: test.replace('class BindingServiceTest {', `class BindingServiceTest {\n${service}`) },
  ]
}

/** @param {string} service */
function csharpConfigurationTests(service) {
  return [
    { name: 'client/tests/BindingTestSetup.cs', code: '[TestSetup]\npublic sealed class BindingTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n' },
    { name: 'client/tests/BindingServiceTest.cs', code: `[Test(typeof(BindingTestSetup), {"partition":"tests"})]\nsealed class BindingServiceTest {\n${service}\n    void testBindsPublishedConfiguration() {\n        var settings = new SettingsService();\n        Assert.equals("Catalog", settings.title);\n        Assert.equals(20, settings.pageSize);\n    }\n}\n` },
  ]
}

/** @param {string} service */
function rustConfigurationTests(service) {
  return [
    { name: 'client/tests/BindingTestSetup.rs', code: '#[TestSetup]\nstruct BindingTestSetup {}\nimpl BindingTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }\n}\n' },
    { name: 'client/tests/BindingServiceTest.rs', code: `#[Test(BindingTestSetup, {"partition":"tests"})]\nstruct BindingServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }\nimpl BindingServiceTest {\n    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }\n    fn testBindsPublishedConfiguration(&self) {\n${service}\n        let settings = SettingsService::new();\n        Assert::equals("Catalog", settings.title);\n        Assert::equals(20, settings.pageSize);\n    }\n}\n` },
  ]
}
