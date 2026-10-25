import { spawn } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { setTimeout as sleep } from 'node:timers/promises'

/** @typedef {{kind:'css',value:string}|{kind:'role',role:string,name?:string|RegExp,exact?:boolean}|{kind:'label',text:string|RegExp,exact?:boolean}|{kind:'text',text:string|RegExp,exact?:boolean}|{kind:'nth',index:number}} LocatorStep */
/** @typedef {{driver:string,args:string[],capabilities:Record<string,unknown>,name:string,engine:string}} DriverPlan */

/**
 * A Playwright-shaped browser over a W3C WebDriver server (safaridriver, or WebKitWebDriver as a
 * stand-in). It covers the calls the Tac web gates make; any other call throws BrandedUnsupported
 * so a gate never passes on a call that did not reach the branded browser.
 */
export class BrandedUnsupported extends Error {
  static marker = 'branded browser cannot'

  /** @param {string} browser @param {string} call @param {string} [why] */
  constructor(browser, call, why = 'the WebDriver protocol has no equivalent') {
    super(`${BrandedUnsupported.marker} run ${call} in ${browser}: ${why}`)
    this.name = 'BrandedUnsupported'
  }
}

export class WebDriverSession {
  static element = 'element-6066-11e4-a52e-4f735466cecf'
  static startMilliseconds = 60_000
  static stopMilliseconds = 5_000
  static pollMilliseconds = 100

  /** @param {DriverPlan} plan */
  static async start(plan) {
    const port = await WebDriverSession.freePort()
    const child = spawn(plan.driver, plan.args.map((argument) => argument.replace('{port}', String(port))), { stdio: ['ignore', 'pipe', 'pipe'] })
    const output = []
    child.stdout.on('data', (chunk) => output.push(String(chunk)))
    child.stderr.on('data', (chunk) => output.push(String(chunk)))
    const base = `http://127.0.0.1:${port}`
    const deadline = Date.now() + WebDriverSession.startMilliseconds
    let ready = false
    while (!ready && Date.now() < deadline && child.exitCode === null) {
      ready = await fetch(`${base}/status`).then((response) => response.ok, () => false)
      if (!ready) await sleep(WebDriverSession.pollMilliseconds)
    }
    if (!ready) {
      child.kill()
      throw new Error(`${plan.driver} did not answer /status within ${WebDriverSession.startMilliseconds} ms: ${output.join('')}`)
    }
    const session = new WebDriverSession(base, child, plan)
    try {
      const created = await session.command('POST', '/session', { capabilities: { alwaysMatch: plan.capabilities } }, false)
      session.id = created.sessionId
      session.capabilities = created.capabilities
    } catch (error) {
      child.kill()
      throw new Error(`${plan.driver} refused a ${plan.name} session (${error.message}). Output: ${output.join('')}`)
    }
    return session
  }

  /** @returns {Promise<number>} */
  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = /** @type {import('node:net').AddressInfo} */ (server.address())
        server.close(() => resolve(port))
      })
    })
  }

  /** @param {string} base @param {import('node:child_process').ChildProcess} child @param {DriverPlan} plan */
  constructor(base, child, plan) {
    this.base = base
    this.child = child
    this.plan = plan
    this.id = ''
    /** @type {Record<string, any>} */
    this.capabilities = {}
    this.window = ''
  }

  /** @param {string} method @param {string} route @param {unknown} [body] @param {boolean} [inSession] */
  async command(method, route, body, inSession = true) {
    const url = `${this.base}${inSession ? `/session/${this.id}` : ''}${route}`
    if (process.env.TAC_BRANDED_TRACE) console.error(`branded: ${method} ${route}`)
    const response = await fetch(url, { method, headers: { 'content-type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) })
    const answer = await response.json().catch(() => ({ value: { error: 'unreadable answer', message: `HTTP ${response.status}` } }))
    if (!response.ok || answer.value?.error) throw new Error(`${method} ${route}: ${answer.value?.error ?? response.status}: ${answer.value?.message ?? ''}`.trim())
    return answer.value
  }

  /** @param {string} handle */
  async focus(handle) {
    if (this.window === handle) return
    await this.command('POST', '/window', { handle })
    this.window = handle
  }

  /** @param {string} script @param {unknown[]} args */
  run(script, args) {
    return this.command('POST', '/execute/async', { script, args })
  }

  async stop() {
    await this.command('DELETE', '').catch(() => {})
    if (this.child.exitCode !== null || this.child.signalCode !== null) return
    const exited = new Promise((resolve) => this.child.once('exit', resolve))
    this.child.kill()
    const stopped = await Promise.race([exited.then(() => true), sleep(WebDriverSession.stopMilliseconds).then(() => false)])
    if (!stopped) {
      this.child.kill('SIGKILL')
      await exited
    }
  }
}

export class PageProbe {
  static source = String(function tachyonBrandedProbe() {
    const probe = globalThis.__tachyonBrandedProbe ??= { errors: [], console: [], requests: [], seenRequests: 0 }
    if (probe.installed) return
    probe.installed = true
    globalThis.addEventListener('error', (event) => probe.errors.push({ message: event.error?.message ?? event.message, stack: event.error?.stack ?? '' }))
    globalThis.addEventListener('unhandledrejection', (event) => probe.errors.push({ message: String(event.reason?.message ?? event.reason), stack: event.reason?.stack ?? '' }))
    for (const type of ['log', 'info', 'warn', 'error', 'debug']) {
      const original = console[type].bind(console)
      console[type] = (...values) => {
        probe.console.push({ type: type === 'warn' ? 'warning' : type, text: values.map((value) => String(value)).join(' ') })
        original(...values)
      }
    }
  })

  static drain = String(function tachyonBrandedDrain() {
    const probe = globalThis.__tachyonBrandedProbe
    if (!probe) return { errors: [], console: [], requests: [] }
    const entries = performance.getEntriesByType('resource').map((entry) => entry.name)
    const requests = entries.slice(probe.seenRequests)
    probe.seenRequests = entries.length
    const drained = { errors: probe.errors, console: probe.console, requests }
    probe.errors = []
    probe.console = []
    return drained
  })

  static finder = String(function tachyonBrandedFind(steps, scope) {
    const roots = (start) => {
      const found = [start]
      const walk = (node) => {
        if (node.shadowRoot) {
          found.push(node.shadowRoot)
          walk(node.shadowRoot)
        }
        for (const element of node.querySelectorAll('*')) {
          if (element.shadowRoot) {
            found.push(element.shadowRoot)
            walk(element.shadowRoot)
          }
        }
      }
      walk(start)
      return found
    }
    const splitTop = (selector, separator) => {
      const parts = []
      let depth = 0
      let quote = ''
      let current = ''
      for (const character of selector) {
        if (quote) {
          if (character === quote) quote = ''
        } else if (character === '"' || character === "'") quote = character
        else if ('(['.includes(character)) depth += 1
        else if (')]'.includes(character)) depth -= 1
        if (!quote && depth === 0 && separator.test(character)) {
          if (current.trim()) parts.push(current.trim())
          current = ''
          continue
        }
        current += character
      }
      if (current.trim()) parts.push(current.trim())
      return parts
    }
    const deep = (start, selector) => roots(start).flatMap((root) => [...root.querySelectorAll(selector)])
    const css = (start, selector) => {
      const found = []
      for (const alternative of splitTop(selector, /,/)) {
        const combinators = /[>+~]/.test(alternative)
        const parts = combinators ? [alternative] : splitTop(alternative, /\s/)
        let current = [start]
        for (const part of parts) current = [...new Set(current.flatMap((node) => deep(node, part)))]
        found.push(...current)
      }
      const unique = [...new Set(found)]
      return unique.sort((first, second) => (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1))
    }
    const text = (element) => (element.innerText ?? element.textContent ?? '').replace(/\s+/g, ' ').trim()
    const matches = (value, wanted, exact) => {
      if (wanted === undefined || wanted === null) return true
      if (wanted && typeof wanted === 'object' && 'source' in wanted) return new RegExp(wanted.source, wanted.flags).test(value)
      return exact ? value === wanted : value.toLowerCase().includes(String(wanted).toLowerCase())
    }
    const implicitRoles = {
      button: 'button, input[type=button], input[type=submit], input[type=reset], input[type=image], summary, [role=button]',
      link: 'a[href], area[href], [role=link]',
      heading: 'h1, h2, h3, h4, h5, h6, [role=heading]',
      textbox: 'input:not([type]), input[type=text], input[type=email], input[type=tel], input[type=url], input[type=search], textarea, [contenteditable=""], [contenteditable=true], [role=textbox]',
      checkbox: 'input[type=checkbox], [role=checkbox]',
      radio: 'input[type=radio], [role=radio]',
      combobox: 'select, [role=combobox]',
      listitem: 'li, [role=listitem]',
      list: 'ul, ol, [role=list]',
      img: 'img[alt]:not([alt=""]), [role=img]',
      navigation: 'nav, [role=navigation]',
      main: 'main, [role=main]',
      dialog: 'dialog, [role=dialog]',
    }
    const labelText = (element) => {
      const labelled = element.getAttribute('aria-labelledby')
      if (labelled) return labelled.split(/\s+/).map((id) => element.getRootNode().getElementById?.(id) ?? document.getElementById(id)).filter(Boolean).map(text).join(' ')
      return element.getAttribute('aria-label') ?? ''
    }
    const accessibleName = (element) => {
      const named = labelText(element)
      if (named) return named.trim()
      if (element.labels?.length) return [...element.labels].map(text).join(' ')
      if (element instanceof HTMLInputElement && ['button', 'submit', 'reset'].includes(element.type)) return element.value
      if (element instanceof HTMLImageElement) return element.alt
      return text(element) || element.getAttribute('title') || ''
    }
    const hidden = (element) => element.closest?.('[hidden], [aria-hidden=true]') !== null
    const byRole = (start, step) => css(start, implicitRoles[step.role] ?? `[role=${step.role}]`).filter((element) => !hidden(element) && matches(accessibleName(element), step.name, step.exact))
    const byLabel = (start, step) => css(start, 'input, textarea, select, button, meter, output, progress, [contenteditable], [aria-label], [aria-labelledby]').filter((element) => {
      const names = [labelText(element), ...[...(element.labels ?? [])].map(text)].filter(Boolean)
      return names.some((name) => matches(name.trim(), step.text, step.exact))
    })
    const byText = (start, step) => {
      const candidates = css(start, 'body *').filter((element) => matches(text(element), step.text, step.exact))
      return candidates.filter((element) => !candidates.some((other) => other !== element && element.contains(other)))
    }
    let current = [scope ?? document]
    for (const step of steps) {
      if (step.kind === 'nth') {
        const index = step.index < 0 ? current.length + step.index : step.index
        current = current[index] ? [current[index]] : []
        continue
      }
      const finders = { css: (start) => css(start, step.value), role: (start) => byRole(start, step), label: (start) => byLabel(start, step), text: (start) => byText(start, step) }
      current = [...new Set(current.flatMap(finders[step.kind]))]
    }
    return current
  })
}

class BrandedObject {
  static passThrough = new Set(['then', 'toJSON', 'constructor', 'inspect', 'nodeType', 'asymmetricMatch', '$$typeof'])

  /**
   * @template {object} T
   * @param {T} target @param {string} browser @param {string} kind
   * @returns {T}
   */
  static guard(target, browser, kind) {
    return new Proxy(target, {
      get(object, property, receiver) {
        if (typeof property === 'symbol' || property in object || BrandedObject.passThrough.has(property)) return Reflect.get(object, property, receiver)
        return () => {
          throw new BrandedUnsupported(browser, `${kind}.${property}()`)
        }
      },
    })
  }
}

export class BrandedLocator {
  static clickMilliseconds = 1_000

  /** @param {BrandedPage} page @param {LocatorStep[]} steps */
  constructor(page, steps) {
    this.page = page
    this.steps = steps
  }

  /** @param {LocatorStep} step */
  chain(step) {
    return BrandedObject.guard(new BrandedLocator(this.page, [...this.steps, step]), this.page.browserName, 'locator')
  }

  /** @param {string} selector */
  locator(selector) {
    return this.chain({ kind: 'css', value: selector })
  }

  /** @param {string} role @param {{name?:string|RegExp,exact?:boolean}} [options] */
  getByRole(role, options = {}) {
    return this.chain({ kind: 'role', role, name: BrandedLocator.wire(options.name), exact: options.exact })
  }

  /** @param {string|RegExp} text @param {{exact?:boolean}} [options] */
  getByLabel(text, options = {}) {
    return this.chain({ kind: 'label', text: BrandedLocator.wire(text), exact: options.exact })
  }

  /** @param {string|RegExp} text @param {{exact?:boolean}} [options] */
  getByText(text, options = {}) {
    return this.chain({ kind: 'text', text: BrandedLocator.wire(text), exact: options.exact })
  }

  /** @param {string} id */
  getByTestId(id) {
    return this.locator(`[data-testid="${id}"]`)
  }

  /** @param {unknown} value */
  static wire(value) {
    return value instanceof RegExp ? { source: value.source, flags: value.flags } : value
  }

  /** @param {number} index */
  nth(index) {
    return this.chain({ kind: 'nth', index })
  }

  first() {
    return this.nth(0)
  }

  last() {
    return this.nth(-1)
  }

  /** @returns {Promise<unknown[]>} */
  elements() {
    return this.page.evaluateRaw(`return (${PageProbe.finder})(arguments[0])`, [this.steps])
  }

  async one(call) {
    const deadline = Date.now() + this.page.timeout
    let found = await this.elements()
    while (found.length === 0 && Date.now() < deadline) {
      await sleep(WebDriverSession.pollMilliseconds)
      found = await this.elements()
    }
    if (found.length === 0) throw new Error(`${call}: no element matches ${JSON.stringify(this.steps)} within ${this.page.timeout} ms`)
    if (found.length > 1) throw new Error(`${call}: strict mode violation: ${JSON.stringify(this.steps)} matches ${found.length} elements`)
    return found[0]
  }

  async count() {
    return (await this.elements()).length
  }

  /** @param {{state?:'attached'|'detached'|'visible'|'hidden',timeout?:number}} [options] */
  async waitFor(options = {}) {
    const state = options.state ?? 'visible'
    const timeout = options.timeout ?? this.page.timeout
    const deadline = Date.now() + timeout
    const visible = 'return arguments[0].some((element) => { const box = element.getBoundingClientRect(); const style = getComputedStyle(element); return box.width > 0 && box.height > 0 && style.visibility !== "hidden" })'
    for (;;) {
      const found = await this.elements()
      const anyVisible = found.length > 0 && await this.page.evaluateRaw(visible, [found])
      const reached = { attached: found.length > 0, detached: found.length === 0, visible: anyVisible, hidden: !anyVisible }[state]
      if (reached) return
      if (Date.now() > deadline) throw new Error(`locator.waitFor: ${JSON.stringify(this.steps)} did not become ${state} within ${timeout} ms`)
      await sleep(WebDriverSession.pollMilliseconds)
    }
  }

  async click() {
    const element = await this.one('locator.click')
    await this.page.evaluateRaw('arguments[0].scrollIntoView({ block: "center", inline: "center" })', [element])
    const failure = await this.tap(element)
    if (failure) {
      console.error(`branded: the WebDriver click on ${JSON.stringify(this.steps)} did not click (${failure}); clicking through element.click()`)
      await this.page.evaluateRaw('arguments[0].click()', [element])
    }
    await this.page.drain()
  }

  /** @param {unknown} element @returns {Promise<string>} */
  async tap(element) {
    if (this.page.session.plan.capabilities['safari:useSimulator']) return 'a safaridriver tap on the iOS simulator holds its touch-up until the next tap, so it never clicks its target and can click the previous one'
    await this.page.evaluateRaw('const element = arguments[0]; const tap = window.tachyonBrandedTap = { clicked: false }; const listen = (event) => { if (!event.composedPath().includes(element)) return; tap.clicked = true; removeEventListener("click", listen, true) }; addEventListener("click", listen, true)', [element])
    return this.page.session.command('POST', `/element/${element[WebDriverSession.element]}/click`, {}).then(
      async () => (await this.clicked() ? '' : `no click event reached it within ${BrandedLocator.clickMilliseconds} ms`),
      (error) => `it failed: ${error.message}`,
    )
  }

  /** @returns {Promise<boolean>} */
  async clicked() {
    const deadline = Date.now() + BrandedLocator.clickMilliseconds
    for (;;) {
      const clicked = await this.page.evaluateRaw('return window.tachyonBrandedTap === undefined || window.tachyonBrandedTap.clicked', []).catch(() => true)
      if (clicked || Date.now() > deadline) return clicked
      await sleep(WebDriverSession.pollMilliseconds)
    }
  }

  /** @param {string} value */
  async fill(value) {
    const element = await this.one('locator.fill')
    const id = element[WebDriverSession.element]
    try {
      await this.page.session.command('POST', `/element/${id}/clear`, {})
      if (value !== '') await this.page.session.command('POST', `/element/${id}/value`, { text: value })
      const typed = await this.page.evaluateRaw('return !("value" in arguments[0]) || arguments[0].value === arguments[1]', [element, value])
      if (!typed) throw new Error('the keys did not change its value')
    } catch (error) {
      console.error(`branded: WebDriver typing into ${JSON.stringify(this.steps)} failed (${error.message}); filling it the way Playwright's fill does (focus, set the value, dispatch input)`)
      await this.page.evaluateRaw('const [element, value] = arguments; element.focus(); element.value = value; element.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: value })); element.dispatchEvent(new Event("change", { bubbles: true }))', [element, value])
    }
    await this.page.drain()
  }

  /** @param {string} value */
  async pressSequentially(value) {
    const element = await this.one('locator.pressSequentially')
    await this.page.session.command('POST', `/element/${element[WebDriverSession.element]}/value`, { text: value })
  }

  async focus() {
    await this.evaluate((element) => element.focus())
  }

  async check() {
    const checked = await this.isChecked()
    if (!checked) await this.click()
  }

  /** @param {string|string[]} value */
  async selectOption(value) {
    const wanted = [value].flat()
    return this.evaluate((element, values) => {
      for (const option of element.options) option.selected = values.includes(option.value) || values.includes(option.label)
      element.dispatchEvent(new Event('input', { bubbles: true }))
      element.dispatchEvent(new Event('change', { bubbles: true }))
      return [...element.selectedOptions].map((option) => option.value)
    }, wanted)
  }

  textContent() {
    return this.evaluate((element) => element.textContent)
  }

  innerText() {
    return this.evaluate((element) => element.innerText)
  }

  innerHTML() {
    return this.evaluate((element) => element.innerHTML)
  }

  inputValue() {
    return this.evaluate((element) => element.value)
  }

  /** @param {string} name */
  getAttribute(name) {
    return this.evaluate((element, attribute) => element.getAttribute(attribute), name)
  }

  isChecked() {
    return this.evaluate((element) => Boolean(element.checked ?? element.getAttribute('aria-checked') === 'true'))
  }

  async isVisible() {
    const found = await this.elements()
    if (found.length === 0) return false
    return this.page.evaluateRaw('const box = arguments[0].getBoundingClientRect(); return box.width > 0 && box.height > 0 && getComputedStyle(arguments[0]).visibility !== "hidden"', [found[0]])
  }

  async allTextContents() {
    return this.evaluateAll((elements) => elements.map((element) => element.textContent ?? ''))
  }

  async allInnerTexts() {
    return this.evaluateAll((elements) => elements.map((element) => element.innerText ?? ''))
  }

  /** @param {Function|string} code @param {unknown} [arg] */
  async evaluate(code, arg) {
    const element = await this.one('locator.evaluate')
    return this.page.evaluateWith(code, [element, arg])
  }

  /** @param {Function|string} code @param {unknown} [arg] */
  async evaluateAll(code, arg) {
    return this.page.evaluateWith(code, [await this.elements(), arg])
  }

  async elementHandle() {
    const element = await this.one('locator.elementHandle')
    const slot = await this.page.evaluateRaw('const handles = globalThis.__tachyonBrandedHandles ??= []; handles.push(arguments[0]); return handles.length - 1', [element])
    return new BrandedHandle(this.page, element, slot)
  }
}

export class BrandedHandle {
  /**
   * An element handle keeps its element in a page-side slot, because WebDriver refuses a
   * reference to an element once it has left the document, and gates check exactly that.
   * @param {BrandedPage} page @param {unknown} value @param {number} [slot]
   */
  constructor(page, value, slot) {
    this.page = page
    this.value = value
    this.slot = slot
  }

  /** @param {Function|string} code @param {unknown} [arg] */
  evaluate(code, arg) {
    if (this.slot === undefined) return this.page.evaluateWith(code, [this.value, arg])
    return this.page.evaluateWith(`([slot, given]) => (${String(code)})(globalThis.__tachyonBrandedHandles[slot], given)`, [[this.slot, arg]])
  }

  async jsonValue() {
    return this.value
  }

  async dispose() {}
}

export class BrandedPage {
  static defaultTimeout = 30_000
  static quietMilliseconds = 500

  /** @param {BrandedContext} context @param {string} handle */
  constructor(context, handle) {
    this.context_ = context
    this.session = context.browser_.session
    this.browserName = context.browser_.plan.name
    this.handle = handle
    this.timeout = BrandedPage.defaultTimeout
    this.closed = false
    /** @type {Map<string, Set<Function>>} */
    this.listeners = new Map()
    this.warned = new Set()
  }

  context() {
    return this.context_
  }

  /** @param {string} event @param {Function} listener */
  on(event, listener) {
    if (event === 'requestfailed' && !this.warned.has(event)) {
      this.warned.add(event)
      console.error(`branded: ${this.browserName} over WebDriver reports no failed requests; requestfailed listeners hear nothing`)
    }
    if (!this.listeners.has(event)) this.listeners.set(event, new Set())
    this.listeners.get(event)?.add(listener)
    return this
  }

  /** @param {string} event @param {Function} listener */
  off(event, listener) {
    this.listeners.get(event)?.delete(listener)
    return this
  }

  /** @param {string} event @param {Function} listener */
  once(event, listener) {
    const wrapped = (...values) => {
      this.off(event, wrapped)
      listener(...values)
    }
    return this.on(event, wrapped)
  }

  /** @param {string} event @param {unknown} value */
  emit(event, value) {
    for (const listener of this.listeners.get(event) ?? []) listener(value)
  }

  /** @param {number} timeout */
  setDefaultTimeout(timeout) {
    this.timeout = timeout
  }

  /** @param {number} timeout */
  setDefaultNavigationTimeout(timeout) {
    this.timeout = timeout
  }

  /** @param {string} script @param {unknown[]} args */
  async evaluateRaw(script, args) {
    await this.session.focus(this.handle)
    return this.session.command('POST', '/execute/sync', { script, args })
  }

  /** @param {Function|string} code @param {unknown[]} args */
  async evaluateWith(code, args) {
    await this.session.focus(this.handle)
    const body = typeof code === 'function' ? `(${String(code)})(...arguments[0])` : `((value) => typeof value === "function" ? value(...arguments[0]) : value)((${code}))`
    const script = `const done = arguments[arguments.length - 1]; Promise.resolve().then(() => ${body}).then((value) => done({ value, missing: value === undefined }), (error) => done({ error: String(error && error.stack || error), message: String(error && error.message || error) }))`
    const answer = await this.session.run(script, [args])
    await this.drain()
    if (answer?.error !== undefined) {
      const error = new Error(answer.message)
      error.stack = answer.error
      throw error
    }
    return answer?.missing ? undefined : answer?.value
  }

  /** @param {Function|string} code @param {unknown} [arg] */
  evaluate(code, arg) {
    return this.evaluateWith(code, [arg])
  }

  /** @param {Function|string} code @param {unknown} [arg] */
  async evaluateHandle(code, arg) {
    return new BrandedHandle(this, await this.evaluate(code, arg))
  }

  /** @param {Function|string} code @param {unknown} [arg] @param {{timeout?:number,polling?:number}} [options] */
  async waitForFunction(code, arg, options = {}) {
    const timeout = options.timeout ?? this.timeout
    const deadline = Date.now() + timeout
    for (;;) {
      const value = await this.evaluate(code, arg)
      if (value) return new BrandedHandle(this, value)
      if (Date.now() > deadline) throw new Error(`page.waitForFunction: ${String(code).slice(0, 200)} stayed false for ${timeout} ms`)
      await sleep(typeof options.polling === 'number' ? options.polling : WebDriverSession.pollMilliseconds)
    }
  }

  /** @param {number} milliseconds */
  waitForTimeout(milliseconds) {
    return sleep(milliseconds)
  }

  async drain() {
    if (this.closed) return
    const drained = await this.evaluateRaw(`return (${PageProbe.drain})()`, []).catch(() => ({ errors: [], console: [], requests: [] }))
    for (const failure of drained.errors) {
      const error = new Error(failure.message)
      error.stack = failure.stack || failure.message
      this.emit('pageerror', error)
    }
    for (const message of drained.console) this.emit('console', { type: () => message.type, text: () => message.text })
    for (const url of drained.requests) this.emit('request', { url: () => url, method: () => 'GET', resourceType: () => 'other' })
  }

  async settle(waitUntil) {
    await this.evaluateRaw(`(${PageProbe.source})()`, [])
    if (waitUntil !== 'networkidle') return
    const deadline = Date.now() + this.timeout
    let last = -1
    let quietSince = Date.now()
    while (Date.now() < deadline) {
      const count = await this.evaluateRaw('return performance.getEntriesByType("resource").length', [])
      if (count !== last) {
        last = count
        quietSince = Date.now()
      } else if (Date.now() - quietSince >= BrandedPage.quietMilliseconds) return
      await sleep(WebDriverSession.pollMilliseconds)
    }
  }

  /** @param {string} url @param {{waitUntil?:string,timeout?:number}} [options] */
  async goto(url, options = {}) {
    await this.session.focus(this.handle)
    await this.session.command('POST', '/url', { url })
    await this.settle(options.waitUntil)
    await this.drain()
    const status = await this.evaluateRaw('return performance.getEntriesByType("navigation")[0]?.responseStatus ?? null', [])
    return BrandedObject.guard({
      url: () => url,
      status: () => {
        if (typeof status !== 'number') throw new BrandedUnsupported(this.browserName, 'response.status()', 'this Safari exposes no navigation responseStatus')
        return status
      },
      ok: () => typeof status !== 'number' || (status >= 200 && status < 300),
    }, this.browserName, 'response')
  }

  /** @param {{waitUntil?:string}} [options] */
  async reload(options = {}) {
    await this.session.focus(this.handle)
    await this.session.command('POST', '/refresh', {})
    await this.settle(options.waitUntil)
    await this.drain()
  }

  async url() {
    await this.session.focus(this.handle)
    return this.session.command('GET', '/url')
  }

  async title() {
    await this.session.focus(this.handle)
    return this.session.command('GET', '/title')
  }

  content() {
    return this.evaluate(() => document.documentElement.outerHTML)
  }

  async bringToFront() {
    await this.session.focus(this.handle)
  }

  /** @param {{path?:string}} [options] */
  async screenshot(options = {}) {
    await this.session.focus(this.handle)
    const image = Buffer.from(await this.session.command('GET', '/screenshot'), 'base64')
    if (options.path) writeFileSync(options.path, image)
    return image
  }

  /** @param {string} selector */
  locator(selector) {
    return BrandedObject.guard(new BrandedLocator(this, []), this.browserName, 'locator').locator(selector)
  }

  /** @param {string} role @param {{name?:string|RegExp,exact?:boolean}} [options] */
  getByRole(role, options) {
    return BrandedObject.guard(new BrandedLocator(this, []), this.browserName, 'locator').getByRole(role, options)
  }

  /** @param {string|RegExp} text @param {{exact?:boolean}} [options] */
  getByLabel(text, options) {
    return BrandedObject.guard(new BrandedLocator(this, []), this.browserName, 'locator').getByLabel(text, options)
  }

  /** @param {string|RegExp} text @param {{exact?:boolean}} [options] */
  getByText(text, options) {
    return BrandedObject.guard(new BrandedLocator(this, []), this.browserName, 'locator').getByText(text, options)
  }

  /** @param {string} id */
  getByTestId(id) {
    return this.locator(`[data-testid="${id}"]`)
  }

  /** @param {string} selector @param {{state?:'attached'|'detached'|'visible'|'hidden',timeout?:number}} [options] */
  async waitForSelector(selector, options) {
    await this.locator(selector).first().waitFor(options)
  }

  /** @param {string} selector */
  click(selector) {
    return this.locator(selector).click()
  }

  /** @param {string} selector @param {string} value */
  fill(selector, value) {
    return this.locator(selector).fill(value)
  }

  /** @param {string} selector */
  focus(selector) {
    return this.locator(selector).focus()
  }

  /** @param {string} selector */
  textContent(selector) {
    return this.locator(selector).first().textContent()
  }

  /** @param {string} selector */
  innerText(selector) {
    return this.locator(selector).first().innerText()
  }

  /** @param {string} selector */
  inputValue(selector) {
    return this.locator(selector).inputValue()
  }

  /** @param {string} selector @param {string} name */
  getAttribute(selector, name) {
    return this.locator(selector).first().getAttribute(name)
  }

  /** @param {string} selector */
  isVisible(selector) {
    return this.locator(selector).isVisible()
  }

  /** @param {string} selector @param {string|string[]} value */
  selectOption(selector, value) {
    return this.locator(selector).selectOption(value)
  }

  /** @param {string} selector @param {Function|string} code @param {unknown} [arg] */
  $eval(selector, code, arg) {
    return this.locator(selector).first().evaluate(code, arg)
  }

  /** @param {string} selector @param {Function|string} code @param {unknown} [arg] */
  $$eval(selector, code, arg) {
    return this.locator(selector).evaluateAll(code, arg)
  }

  isClosed() {
    return this.closed
  }

  async close() {
    if (this.closed) return
    this.closed = true
    await this.context_.forget(this)
  }
}

export class BrandedContext {
  /** @param {BrandedBrowser} browser */
  constructor(browser) {
    this.browser_ = browser
    /** @type {BrandedPage[]} */
    this.open = []
    this.timeout = BrandedPage.defaultTimeout
  }

  browser() {
    return this.browser_
  }

  pages() {
    return [...this.open]
  }

  /** @param {number} timeout */
  setDefaultTimeout(timeout) {
    this.timeout = timeout
  }

  /** @param {number} timeout */
  setDefaultNavigationTimeout(timeout) {
    this.timeout = timeout
  }

  async newPage() {
    const page = BrandedObject.guard(new BrandedPage(this, await this.browser_.window()), this.browser_.plan.name, 'page')
    page.setDefaultTimeout(this.timeout)
    await page.goto('about:blank')
    this.open.push(page)
    return page
  }

  /** @param {BrandedPage} page */
  async forget(page) {
    this.open = this.open.filter((open) => open !== page)
    await this.browser_.release(page.handle)
  }

  async close() {
    for (const page of [...this.open]) await page.close()
    this.browser_.contextClosed(this)
  }
}

export class BrandedBrowser {
  /** @param {WebDriverSession} session @param {string} version */
  constructor(session, version) {
    this.session = session
    this.plan = session.plan
    this.version_ = version
    /** @type {BrandedContext|undefined} */
    this.current = undefined
    /** @type {string[]} */
    this.spare = []
    /** @type {string[]} */
    this.used = []
  }

  /** @param {DriverPlan} plan */
  static async launch(plan) {
    const session = await WebDriverSession.start(plan)
    const handle = await session.command('GET', '/window')
    session.window = handle
    const caps = session.capabilities
    const userAgent = await session.command('POST', '/execute/sync', { script: 'return navigator.userAgent', args: [] })
    const version = `${caps.browserName ?? plan.name} ${caps.browserVersion ?? ''}`.trim()
    const browser = BrandedObject.guard(new BrandedBrowser(session, version), plan.name, 'browser')
    browser.spare.push(handle)
    browser.userAgent = userAgent
    return browser
  }

  version() {
    return this.version_
  }

  browserType() {
    return { name: () => this.plan.engine }
  }

  isConnected() {
    return true
  }

  contexts() {
    return this.current ? [this.current] : []
  }

  async newContext() {
    if (this.current) throw new BrandedUnsupported(this.plan.name, 'a second browser.newContext() while one is open', 'Safari runs one automation session at a time')
    this.current = BrandedObject.guard(new BrandedContext(this), this.plan.name, 'context')
    return this.current
  }

  async newPage() {
    const context = await this.newContext()
    const page = await context.newPage()
    const close = page.close.bind(page)
    page.close = async () => {
      await close()
      await context.close()
    }
    return page
  }

  /** @returns {Promise<string>} */
  async window() {
    const handle = this.spare.shift() ?? (await this.session.command('POST', '/window/new', { type: 'tab' })).handle
    this.used.push(handle)
    return handle
  }

  /** @param {string} handle */
  async release(handle) {
    this.used = this.used.filter((used) => used !== handle)
    await this.session.focus(handle)
    if (this.used.length + this.spare.length === 0) {
      await this.session.command('POST', '/url', { url: 'about:blank' }).catch(() => {})
      this.spare.push(handle)
      return
    }
    await this.session.command('DELETE', '/window').catch(() => {})
    this.session.window = ''
  }

  /** @param {BrandedContext} context */
  contextClosed(context) {
    if (this.current === context) this.current = undefined
  }

  async close() {
    if (this.current) await this.current.close()
    await this.session.stop()
  }
}
