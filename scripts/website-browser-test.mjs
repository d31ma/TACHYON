#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { withServer } from './release/server-probe.mjs';

class WebsiteTest {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static website = path.join(WebsiteTest.repo, 'website');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty');
}

/**
 * @param {import('playwright').Page} page
 * @returns {string[]}
 */
function collectPageErrors(page) {
  /** @type {string[]} */
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error') errors.push(`console: ${message.text()}`);
  });
  page.on('requestfailed', request => errors.push(`request: ${request.url()} (${request.failure()?.errorText ?? 'failed'})`));
  page.on('response', response => {
    if (response.status() >= 400) errors.push(`response: ${response.url()} (${response.status()})`);
  });
  return errors;
}

/**
 * @param {import('playwright').Page} page
 * @param {number} width
 * @param {string} route
 * @param {string[]} errors
 */
async function openSearch(page, width, route, errors) {
  const documentText = await page.locator('body').innerText();
  if (width < 500) {
    await page.waitForFunction(() => document.querySelector('[data-tachyon-component="site-search"][data-tachyon-active="true"]'));
    await page.keyboard.press('Control+k');
    return;
  }
  await page.getByRole('button', { name: 'Search documentation', exact: true }).click().catch(error => {
    throw new Error(`${width}px ${route}: search button unavailable; body: ${(documentText || '<empty>').slice(0, 500)}; browser errors: ${errors.join('; ')}`, { cause: error });
  });
}

/**
 * @param {import('playwright').Page} page
 * @param {number} width
 * @param {string} route
 */
async function checkSearchTyping(page, width, route) {
  const input = page.getByRole('searchbox', { name: 'Search documentation', exact: true });
  await input.waitFor();
  await page.waitForFunction(() => document.activeElement?.id === 'site-search-input');
  const original = await input.elementHandle();
  await input.pressSequentially('native', { delay: 40 });
  assert.equal(await input.inputValue(), 'native', `${width}px ${route}: uninterrupted search typing`);
  await page.waitForFunction(() => /\d+ results?/.test(document.querySelector('.search-panel [aria-live]')?.textContent || ''));
  assert.equal(await input.evaluate((element, previous) => document.activeElement === element && element === previous, original), true,
    'search result updates retain the actual focused input through custom-element slots');
  await input.evaluate(element => /** @type {HTMLInputElement} */ (element).setSelectionRange(2, 2));
  await input.pressSequentially('X', { delay: 40 });
  await page.waitForFunction(() => /Nothing matches/.test(document.querySelector('.search-panel [aria-live]')?.textContent || ''));
  assert.equal(await input.inputValue(), 'naXtive');
  assert.equal(await input.evaluate(element => /** @type {HTMLInputElement} */ (element).selectionStart), 3, 'results must not move caret to the end');
  await input.fill('native ');
  await input.pressSequentially('app', { delay: 0 });
  await page.waitForFunction(() => !/Searching/.test(document.querySelector('.search-panel [aria-live]')?.textContent || ''));
  assert.equal(await input.inputValue(), 'native app', 'space and rapid keystrokes survive async renders');
  await input.press('Escape');
  await page.getByRole('dialog', { name: 'Search documentation' }).waitFor({ state: 'detached' });
  if (width >= 500) {
    await page.waitForFunction(() => document.activeElement?.matches('.site-search__trigger button'));
  }
  await original?.dispose();
}

/**
 * @param {import('playwright').Browser} activeBrowser
 * @param {string} origin
 * @param {number} width
 */
async function checkViewport(activeBrowser, origin, width) {
  const page = await activeBrowser.newPage({ viewport: { width, height: 900 }, hasTouch: width < 500 });
  const errors = collectPageErrors(page);
  page.setDefaultTimeout(10000);
  for (const route of ['/', '/docs/', '/docs/features/companion-publish/']) {
    const response = await page.goto(`${origin}${route}`);
    assert.ok(response, `${width}px ${route}: navigation response`);
    assert.equal(response.status(), 200, `${width}px ${route}: page is served`);
    await openSearch(page, width, route, errors);
    await checkSearchTyping(page, width, route);
  }
  assert.deepEqual(errors, [], `${width}px: no uncaught browser errors`);
  await page.close();
}

async function main() {
  const output = mkdtempSync(path.join(tmpdir(), 'tachyon-website-input-'));
  /** @type {import('playwright').Browser | undefined} */
  let browser;
  try {
    const build = spawnSync(WebsiteTest.binary, ['build', WebsiteTest.website, '--out-dir', output], { encoding: 'utf8', timeout: 120000 });
    assert.equal(build.status, 0, build.stderr || build.stdout);
    const activeBrowser = await chromium.launch();
    browser = activeBrowser;
    await withServer(WebsiteTest.binary, ['preview', WebsiteTest.website, '--static', '--out-dir', output, '--port', '0'], async request => {
      const origin = new URL((await request('/')).url).origin;
      for (const width of [1440, 390]) await checkViewport(activeBrowser, origin, width);
    });
    console.log('Website browser input gate passed: desktop/mobile, home/docs/features, continuous typing, custom slots, middle caret, spaces, rapid input, Escape.');
  } finally {
    if (browser) await browser.close();
    rmSync(output, { recursive: true, force: true });
  }
}

await main();
