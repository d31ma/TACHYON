import { createRequire } from 'node:module'
import path from 'node:path'
import process from 'node:process'

class Sweep {
  static origin = process.env.SITE_ORIGIN ?? 'http://127.0.0.1:4700'

  static paths = ['/', '/docs', '/docs/features',
    '/docs/features/companion-publish', '/docs/install',
    '/docs/pages', '/docs/devices', '/docs/controllers', '/docs/platform-apis',
    '/docs/cookbook']

  static widths = [320, 360, 375, 414, 480, 540, 600, 700, 768, 820, 900, 1024,
    1180, 1280, 1440, 1600, 1920, 2560]

  /** @returns {string[]} */
  static selectedPaths() {
    const selected = process.env.SITE_PATHS?.split(',') ?? Sweep.paths
    if (!selected.length || selected.some(route => !Sweep.paths.includes(route))) {
      throw new Error('SITE_PATHS must be a comma-separated selection of the existing responsive routes')
    }
    return [...new Set(selected)]
  }

  /** @returns {number[]} */
  static selectedWidths() {
    const selected = process.env.SITE_WIDTHS?.split(',').map(Number) ?? Sweep.widths
    if (!selected.length || selected.some(width => !Number.isInteger(width) || width < 320 || width > 2560)) {
      throw new Error('SITE_WIDTHS must contain integer viewport widths from 320 through 2560')
    }
    return [...new Set(selected)].sort((left, right) => left - right)
  }

  /** @returns {string[]} */
  static selectedThemes() {
    const selected = process.env.SITE_THEMES?.split(',') ?? ['dark', 'light']
    if (!selected.length || selected.some(theme => !['dark', 'light'].includes(theme))) {
      throw new Error('SITE_THEMES must select dark, light, or both')
    }
    return [...new Set(selected)]
  }

  /** @returns {import('playwright').BrowserType} */
  static browser() {
    const dependencies = process.env.TAC_BROWSER_DEPENDENCIES
    const load = createRequire(import.meta.url)
    return load(dependencies ? path.resolve(dependencies, 'playwright') : 'playwright').chromium
  }
}

function probe() {
  const root = document.documentElement
  const viewport = root.clientWidth

  const overflowing = []
  for (const element of document.querySelectorAll('body *')) {
    if (element.getBoundingClientRect().width <= viewport + 1) continue
    overflowing.push(`${element.tagName.toLowerCase()}.${String(element.className).slice(0, 30)}`)
  }

  const smallTargets = []
  for (const element of document.querySelectorAll('.w-btn, w-btn, w-tab, .w-chip, button, a[href]')) {
    if (element.closest('p')) continue
    const rendered = element.querySelector(':scope > .w-btn, :scope > .w-tab, :scope > .w-chip') ?? element
    const box = rendered.getBoundingClientRect()
    if (box.width > 0 && box.height > 0 && box.height < 24) {
      smallTargets.push(`${element.tagName.toLowerCase()}:${element.textContent.trim().slice(0, 18)}`)
    }
  }

  /** @param {string} selector @returns {{count: number, ragged: number}} */
  const cardHeights = (selector) => {
    let count = 0
    let ragged = 0
    for (const grid of document.querySelectorAll('.grid-auto--even, .catalogue, .pillars')) {
      const heights = [...grid.querySelectorAll(selector)].map((card) => Math.round(card.getBoundingClientRect().height))
      if (heights.length <= 1) continue
      count += heights.length
      if (new Set(heights).size > 1) ragged += 1
    }
    return { count, ragged }
  }

  const longLines = []
  for (const paragraph of document.querySelectorAll('p')) {
    const text = paragraph.textContent.trim()
    if (text.length < 90) continue
    const size = parseFloat(getComputedStyle(paragraph).fontSize)
    const characters = paragraph.getBoundingClientRect().width / (size * 0.5)
    if (characters > 92) longLines.push(text.slice(0, 24))
  }

  return {
    scrollWidth: root.scrollWidth,
    viewport,
    overflow: [...new Set(overflowing)].slice(0, 3),
    smallTargets: [...new Set(smallTargets)].slice(0, 3),
    cards: cardHeights('.catalogue__card, .pillar, .feature-card'),
    longLines: [...new Set(longLines)].slice(0, 2),
  }
}

/** @param {ReturnType<typeof probe>} report @param {boolean} coarse @returns {string[]} */
function problemsIn(report, coarse) {
  const problems = []
  if (report.scrollWidth > report.viewport + 1) {
    problems.push(`overflows to ${report.scrollWidth}px (${report.overflow.join(' ') || 'unattributed'})`)
  }
  if (coarse && report.smallTargets.length) {
    problems.push(`tap targets under 24px: ${report.smallTargets.join(' ')}`)
  }
  if (report.cards.ragged) {
    problems.push(`${report.cards.ragged} ragged card grid(s)`)
  }
  if (report.longLines.length) {
    problems.push(`line measure over 92ch: ${report.longLines.join(' | ')}`)
  }
  return problems
}

/**
 * @param {import('playwright').Browser} browser
 * @param {string} theme
 * @param {number} width
 * @param {{checks: number, failures: string[]}} summary
 * @returns {Promise<void>}
 */
async function sweepWidth(browser, theme, width, summary) {
  const coarse = width <= 820
  const context = await browser.newContext({
    viewport: { width, height: 900 },
    hasTouch: coarse,
    isMobile: coarse,
    deviceScaleFactor: 1,
  })
  await context.addInitScript((/** @type {string} */ value) => localStorage.setItem('w-theme', value), theme)
  const page = await context.newPage()

  for (const path of Sweep.selectedPaths()) {
    await page.goto(`${Sweep.origin}${path}`, { waitUntil: 'networkidle' })
    await page.waitForTimeout(400)
    const report = await page.evaluate(probe)
    summary.checks += 1

    const problems = problemsIn(report, coarse)
    if (problems.length) {
      summary.failures.push(`${theme} ${String(width).padStart(4)} ${path}`)
      console.log(`FAIL ${theme.padEnd(5)} ${String(width).padStart(4)} ${path.padEnd(20)} ${problems.join('; ')}`)
    }
  }
  await context.close()
}

async function main() {
  const widths = Sweep.selectedWidths()
  const paths = Sweep.selectedPaths()
  const themes = Sweep.selectedThemes()
  const browser = await Sweep.browser().launch()
  const summary = { checks: 0, failures: [] }

  for (const theme of themes) {
    for (const width of widths) await sweepWidth(browser, theme, width, summary)
    console.log(`  swept ${widths.length} widths x ${paths.length} routes in ${theme}`)
  }

  await browser.close()
  console.log(`\n${summary.checks} checks across ${widths[0]}–${widths.at(-1)}px`)
  if (summary.failures.length) {
    console.error(`responsive gate failed: ${summary.failures.length} of ${summary.checks}`)
    process.exit(1)
  }
  console.log('PASS: selected routes hold at selected widths and themes')
}

await main()
