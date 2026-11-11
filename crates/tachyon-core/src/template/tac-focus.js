class TacFocus {
  static namePattern = /^[A-Za-z][A-Za-z0-9_-]{0,63}$/
  static mostWaiting = 16
  static mostTraps = 16
  static tabbable = 'a[href], area[href], button, input, select, textarea, iframe, summary, audio[controls], video[controls], [contenteditable], [tabindex]'
  static waiting = []
  static traps = []
  static page = Object.freeze({ render: () => null, views: () => [], owner: () => null })
  static listening = false
  static api = Object.freeze({
    move: (target) => TacFocus.ask('move', target),
    trap: (container) => TacFocus.ask('trap', container),
    release: () => TacFocus.ask('release', null),
  })

  static connect(page) {
    TacFocus.page = page
    return true
  }

  static owned(owner) {
    return Object.freeze({
      move: (target) => TacWebApis.calling(owner, () => TacFocus.api.move(target)),
      trap: (container) => TacWebApis.calling(owner, () => TacFocus.api.trap(container)),
      release: () => TacWebApis.calling(owner, () => TacFocus.api.release()),
    })
  }

  static ask(call, name) {
    const owner = TacFocus.#caller()
    TacFocus.#checkName(call, name)
    TacFocus.#views(owner)
    TacFocus.#room()
    TacFocus.waiting.push({ call, name, owner, opener: TacFocus.#active() })
    TacFocus.#listen()
    Promise.resolve().then(() => TacFocus.page.render()).catch(() => null)
    return null
  }

  static settle() {
    for (const request of TacFocus.waiting.splice(0)) TacFocus.#settleOne(request)
  }

  static #settleOne(request) {
    try {
      TacFocus.#run(request)
    } catch (error) {
      TacWebApis.reply(request.owner, ['BrowserCallFailed', { api: 'focus', call: request.call, message: TacWebApis.message(error) }])
    }
  }

  static #run(request) {
    switch (request.call) {
      case 'move': return TacFocus.#move(request)
      case 'trap': return TacFocus.#trap(request)
      default: return TacFocus.#release(request)
    }
  }

  static #move({ owner, name }) {
    TacFocus.#focus(TacFocus.#one(owner, name), name)
  }

  static #trap({ owner, name, opener }) {
    TacFocus.#trapRoom()
    const container = TacFocus.#one(owner, name)
    TacFocus.#focus(TacFocus.#first(container), name)
    TacFocus.traps.push({ owner, name, container, opener })
  }

  static #release({ owner }) {
    const trap = TacFocus.#latestTrap(owner)
    TacFocus.traps.splice(TacFocus.traps.indexOf(trap), 1)
    TacFocus.#returnTo(trap)?.focus()
  }

  static #caller() {
    const owner = TacWebApis.caller
    if (!owner) throw new Error('Focus needs the service that calls it. Call it from a service method.')
    return owner
  }

  static #checkName(call, name) {
    if (call !== 'release' && (typeof name !== 'string' || !TacFocus.namePattern.test(name))) throw new TypeError('A focus name is a letter, then up to 63 letters, digits, "-" or "_". Put it in data-focus="..." on the element.')
  }

  static #views(owner) {
    const views = TacFocus.page.views(owner)
    if (!views.length) throw new Error('Focus works only for a service that a shown component uses.')
    return views
  }

  static #room() {
    if (TacFocus.waiting.length >= TacFocus.mostWaiting) throw new RangeError(`Focus keeps at most ${TacFocus.mostWaiting} requests until the next render.`)
  }

  static #trapRoom() {
    if (TacFocus.traps.length >= TacFocus.mostTraps) throw new RangeError(`Focus keeps at most ${TacFocus.mostTraps} traps. Release one first.`)
  }

  static #latestTrap(owner) {
    const trap = TacFocus.traps.findLast((each) => each.owner === owner)
    if (!trap) throw new Error('This service has no focus trap to release.')
    return trap
  }

  static #one(owner, name) {
    const found = TacFocus.#named(owner, name)
    if (found.length !== 1) throw new Error(TacFocus.#notOne(found.length, name))
    return found[0]
  }

  static #notOne(count, name) {
    if (count) return `More than one element has data-focus="${name}". Give each one its own name.`
    return `No element with data-focus="${name}" is shown by a view of this service.`
  }

  static #named(owner, name) {
    const owners = TacFocus.#views(owner).map((view) => view.owner)
    const elements = TacFocus.#views(owner).flatMap((view) => [...view.host.querySelectorAll('[data-focus]')])
    return [...new Set(elements)].filter((element) => element.getAttribute('data-focus') === name && owners.includes(TacFocus.page.owner(element)))
  }

  static #focus(element, name) {
    element.focus()
    if (!TacFocus.#inside(element, TacFocus.#active())) throw new Error(`The element with data-focus="${name}" cannot take focus. Use a button, a link or a form field, or add tabindex="-1".`)
  }

  static #first(container) {
    return TacFocus.#stops(container)[0] ?? container
  }

  static #returnTo({ owner, opener }) {
    if (opener?.isConnected) return opener
    return TacFocus.#sameName(owner, opener?.getAttribute?.('data-focus'))
  }

  static #sameName(owner, name) {
    if (!name || !TacFocus.page.views(owner).length) return null
    return TacFocus.#named(owner, name)[0] ?? null
  }

  static #active() {
    let active = document.activeElement
    while (active?.shadowRoot?.activeElement) active = active.shadowRoot.activeElement
    return active
  }

  static #listen() {
    if (TacFocus.listening) return
    TacFocus.listening = true
    document.addEventListener('keydown', (event) => TacFocus.#key(event), true)
    document.addEventListener('focusin', (event) => TacFocus.#focusIn(event), true)
  }

  static #key(event) {
    if (TacFocus.#plainTab(event)) TacFocus.#cycle(event, TacFocus.#container())
  }

  static #plainTab(event) {
    return event.key === 'Tab' && !event.altKey && !event.ctrlKey && !event.metaKey
  }

  static #cycle(event, container) {
    const next = container && TacFocus.#wrap(TacFocus.#stops(container), container, event.shiftKey)
    if (next) TacFocus.#jump(event, next)
  }

  static #jump(event, next) {
    event.preventDefault()
    next.focus()
  }

  static #wrap(stops, container, backward) {
    if (!stops.length) return container
    const [edge, other] = backward ? [stops[0], stops.at(-1)] : [stops.at(-1), stops[0]]
    const active = TacFocus.#active()
    return active === edge || !TacFocus.#inside(container, active) ? other : null
  }

  static #focusIn(event) {
    const container = TacFocus.#container()
    const target = event.composedPath?.()[0] ?? event.target
    if (container && !TacFocus.#inside(container, target)) TacFocus.#first(container).focus()
  }

  static #container() {
    const trap = TacFocus.traps.at(-1)
    if (!trap) return null
    if (!trap.container.isConnected) trap.container = TacFocus.#sameName(trap.owner, trap.name)
    return trap.container ?? TacFocus.#drop(trap)
  }

  static #drop(trap) {
    TacFocus.traps.pop()
    return TacFocus.#container()
  }

  static #inside(container, node) {
    let current = node
    while (current && current !== container) current = current.parentNode ?? current.host
    return current === container
  }

  static #stops(container) {
    return [...container.querySelectorAll(TacFocus.tabbable)].filter((element) => TacFocus.#reachable(element))
  }

  static #reachable(element) {
    return element.tabIndex >= 0 && !element.disabled && !element.closest('[inert]') && element.getClientRects().length > 0
  }
}
