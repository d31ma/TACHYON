export class ShellApp {
  static runtime = null
  static plan = null
  static text = null
  static pageEnvironment = null
  static file = '/.tachyon/shell.json'

  static async start(text, runtime) {
    const plan = ShellApp.parsed(text)
    if (!plan?.node || !await ShellApp.sameTachyon(plan, runtime.TacEntityStorage.app)) return null
    ShellApp.runtime = runtime
    ShellApp.text = text
    ShellApp.plan = plan
    runtime.ShellView.serviceFolders.add('/.tachyon/services/')
    ShellApp.join()
    SoftNavigation.start()
    await ShellApp.styles()
    return ShellApp
  }

  static parsed(text) {
    try { return JSON.parse(text || '{}') } catch { return null }
  }

  static async sameTachyon(plan, app) {
    if (app === '/') return true
    const response = await fetch(`${app}.tachyon/app.json`).catch(() => null)
    if (!response?.ok) return true
    const table = await response.json().catch(() => null)
    return table?.tachyon === plan.tachyon
  }

  static join() {
    const { Services } = ShellApp.runtime
    Services.catalog = { ...ShellApp.plan.services, ...Services.catalog }
    Services.saved = { ...ShellApp.plan.saved, ...Services.saved }
  }

  static async styles() {
    const present = new Set([...document.querySelectorAll('link[rel="stylesheet"]')].map((link) => link.href))
    const added = (ShellApp.plan.styles || [])
      .filter((href) => !present.has(new URL(href, location.href).href))
      .map((href) => SoftNavigation.addStyle(href))
    await Promise.all(added)
  }

  static renderPage(environment) {
    ShellApp.pageEnvironment = environment
    return ShellApp.runtime.renderNodes([ShellApp.plan.node], environment, 'shell')
  }

  static renderOutlet(context) {
    const { ClientView, renderNodes } = ShellApp.runtime
    const path = `outlet:${ClientView.plan.route}`
    return renderNodes(ClientView.plan.nodes || [], ShellApp.pageEnvironment, path, { slot: null, namespace: context.namespace })
  }

  static async fetched() {
    const response = await fetch(ShellApp.file)
    return response.ok ? response.text() : null
  }
}

class SoftNavigation {
  static runtimeScript = 'script[type="module"][src$="/.tachyon/tac-client.js"]'
  static current = 0

  static start() {
    document.addEventListener('click', SoftNavigation.click)
    addEventListener('popstate', SoftNavigation.pop)
  }

  static click(event) {
    const link = SoftNavigation.link(event)
    if (!link) return
    event.preventDefault()
    void SoftNavigation.go(new URL(link.href), true)
  }

  static link(event) {
    if (event.defaultPrevented || event.button !== 0) return null
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return null
    const link = event.composedPath().find((node) => node instanceof HTMLAnchorElement && node.hasAttribute('href'))
    if (!link || link.hasAttribute('download')) return null
    if (link.target && link.target !== '_self') return null
    const url = new URL(link.href)
    if (url.origin !== location.origin) return null
    const samePage = url.pathname === location.pathname && url.search === location.search
    return samePage && url.hash ? null : link
  }

  static pop() {
    void SoftNavigation.go(new URL(location.href), false)
  }

  static async go(url, push) {
    const turn = SoftNavigation.current += 1
    const scroll = { x: scrollX, y: scrollY }
    const next = await SoftNavigation.page(url).catch(() => null)
    if (turn !== SoftNavigation.current) return
    if (!next) return SoftNavigation.load(url, push)
    const target = new URL(next.url)
    target.hash = url.hash
    if (push) SoftNavigation.push(target, scroll)
    await SoftNavigation.styles(next.document)
    SoftNavigation.enter(next)
    await SoftNavigation.show()
    SoftNavigation.place(target, push)
  }

  static load(url, push) {
    if (push) location.assign(url.href)
    else location.reload()
  }

  static push(url, scroll) {
    history.replaceState({ ...history.state, tachyonScroll: scroll }, '')
    history.pushState({ tachyonScroll: { x: 0, y: 0 } }, '', url.href)
  }

  static async page(url) {
    const response = await fetch(url.href, { headers: { accept: 'text/html' }, credentials: 'same-origin' })
    if (!response.ok || new URL(response.url).origin !== location.origin) return null
    const parsed = new DOMParser().parseFromString(await response.text(), 'text/html')
    const view = parsed.getElementById('tachyon-view')
    const sameShell = await SoftNavigation.shellText(parsed) === ShellApp.text
    if (!view || !sameShell || !SoftNavigation.knownScripts(parsed)) return null
    if (!await ShellApp.sameTachyon(ShellApp.plan, SoftNavigation.appOf(parsed))) return null
    return { plan: JSON.parse(view.textContent || '{}'), document: parsed, url: response.url }
  }

  static async shellText(parsed) {
    const inline = parsed.getElementById('tachyon-shell')
    if (inline) return inline.textContent
    return SoftNavigation.appOf(parsed) === '/' ? null : ShellApp.fetched()
  }

  static appOf(parsed) {
    const script = parsed.querySelector(SoftNavigation.runtimeScript)
    if (!script) return '/'
    return new URL('../', new URL(script.getAttribute('src'), location.href)).pathname
  }

  static knownScripts(parsed) {
    const loaded = new Set([...document.scripts].map((script) => script.src))
    const needed = [...parsed.head.querySelectorAll('script[src]')].filter((script) => !script.matches(SoftNavigation.runtimeScript))
    return needed.every((script) => loaded.has(new URL(script.getAttribute('src'), location.href).href))
  }

  static async styles(parsed) {
    const present = new Set([...document.querySelectorAll('link[rel="stylesheet"]')].map((link) => link.href))
    const added = [...parsed.querySelectorAll('link[rel="stylesheet"]')]
      .filter((link) => !present.has(new URL(link.getAttribute('href'), location.href).href))
      .map((link) => SoftNavigation.addStyle(link.getAttribute('href')))
    await Promise.all(added)
  }

  static addStyle(href) {
    const link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = href
    const loaded = new Promise((resolve) => { link.onload = resolve; link.onerror = resolve })
    document.head.append(link)
    return loaded
  }

  static enter({ plan, document: parsed }) {
    const { ClientView, Page, Services } = ShellApp.runtime
    SoftNavigation.disposePageServices()
    ClientView.page = plan
    ClientView.plan = plan
    Page.owner = Object.assign(Object.create(null), plan.state)
    Services.catalog = { ...ShellApp.plan.services, ...plan.services }
    Services.saved = { ...ShellApp.plan.saved, ...plan.saved }
    SoftNavigation.enterApp(parsed)
    document.title = parsed.title
  }

  static enterApp(parsed) {
    const script = parsed.querySelector(SoftNavigation.runtimeScript)
    if (!script) return
    const runtime = new URL(script.getAttribute('src'), location.href)
    ShellApp.runtime.ShellView.serviceFolders.add(new URL('./services/', runtime).pathname)
    ShellApp.runtime.TacEntityStorage.app = new URL('../', runtime).pathname
  }

  static disposePageServices() {
    const { Services, disposeSharedInstance } = ShellApp.runtime
    const shell = ShellApp.plan.services || {}
    for (const name of [...Services.singletons.keys()]) {
      if (Object.hasOwn(shell, name)) continue
      const instance = Services.resolved.get(name)
      if (instance) disposeSharedInstance(instance)
      Services.resolved.delete(name)
      Services.singletons.delete(name)
    }
  }

  static show() {
    const { render } = ShellApp.runtime
    if (!document.startViewTransition) return render()
    return document.startViewTransition(() => render()).updateCallbackDone
  }

  static place(url, push) {
    const target = push ? { x: 0, y: 0 } : history.state?.tachyonScroll ?? { x: 0, y: 0 }
    scrollTo(target.x, target.y)
    if (url.hash) document.getElementById(decodeURIComponent(url.hash.slice(1)))?.scrollIntoView()
    SoftNavigation.focus()
  }

  static focus() {
    const heading = document.querySelector('main h1') ?? document.querySelector('h1')
    if (!heading) return
    if (!heading.hasAttribute('tabindex')) heading.setAttribute('tabindex', '-1')
    heading.focus({ preventScroll: true })
  }
}
