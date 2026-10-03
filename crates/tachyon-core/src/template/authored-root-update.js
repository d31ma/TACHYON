/** @typedef {{k: string}} AuthoredPlanNode */
/** @typedef {Element & {tachyonComponent?: object}} AuthoredElement */
/** @typedef {ChildNode & {getAttribute?: Element['getAttribute']}} AuthoredChild */
/** @typedef {Element | DocumentFragment} AuthoredParent */
/** @typedef {{node: AuthoredPlanNode, owner: object, path: string, locals: [string, unknown][], children: AuthoredChild[], attributes: Map<string, string>}} AuthoredRecord */
/** @typedef {{node: AuthoredPlanNode, owner: object, path: string, locals: [string, unknown][], value: string}} AuthoredTextRecord */
/** @typedef {{parent: AuthoredParent, old: AuthoredChild | undefined, fresh: AuthoredChild, previous: AuthoredTextRecord | undefined, next: AuthoredTextRecord | undefined}} AuthoredTextInputs */
/** @typedef {{old: Text, fresh: Text, next: AuthoredTextRecord}} AuthoredTextPair */
/** @typedef {{old: AuthoredElement, fresh: AuthoredElement, previous: AuthoredRecord}} AuthoredRetainedPair */
/** @typedef {AuthoredRetainedPair & {distributed: boolean}} AuthoredPlannedPair */
/** @typedef {{parent: AuthoredParent, previous: AuthoredChild[], desired: AuthoredChild[]}} AuthoredChildLayout */
/** @typedef {{slot: HTMLSlotElement, previous: AuthoredChild[], desired: AuthoredChild[]}} AuthoredSlotGroup */
/** @typedef {{element: Element | null, value: string | undefined}} AuthoredEditingSnapshot */

class AuthoredRootUpdate {
  /** @type {string} */
  static #htmlNamespace = 'http://www.w3.org/1999/xhtml'
  /** @type {string} */
  static #svgNamespace = 'http://www.w3.org/2000/svg'
  /** @type {string} */
  static #componentTag = 'tachyon-component'
  /** @type {string} */
  static #componentKind = 'component'
  /** @type {string} */
  static #editingControls = 'textarea, [contenteditable]'
  /** @type {string} */
  static #defaultSlot = ''
  /** @type {string} */
  static #classAttribute = 'class'
  /** @type {AuthoredParent} */
  #container
  /** @type {AuthoredParent} */
  #source
  /** @type {Element | null | undefined} */
  #active
  /** @type {AuthoredEditingSnapshot | null | undefined} */
  #editingAtStart
  /** @type {Map<AuthoredChild, AuthoredChild>} */
  #retained = new Map()
  /** @type {Set<Element>} */
  #existing = new Set()
  /** @type {Set<Element>} */
  #refused = new Set()
  /** @type {AuthoredPlannedPair[]} */
  #pairs = []
  /** @type {AuthoredTextPair[]} */
  #textPairs = []
  /** @type {Map<AuthoredParent, AuthoredChild[]>} */
  #children = new Map()

  /** @param {AuthoredParent} container @param {AuthoredParent} source @param {Element | null | undefined} active @param {AuthoredEditingSnapshot | null | undefined} editingAtStart @returns {void} */
  constructor(container, source, active, editingAtStart) {
    this.#container = container
    this.#source = source
    this.#active = active
    this.#editingAtStart = editingAtStart
  }

  /** @returns {AuthoredRootUpdate} */
  prepare() {
    const previous = AuthoredRootUpdate.#recordedChildren(this.#container)
    const desired = AuthoredRootUpdate.#recordedChildren(this.#source)
    this.#planChildren(this.#container, desired, previous)
    return this
  }

  /** @param {Element} element @returns {boolean} */
  retains(element) {
    return this.#existing.has(element)
  }

  /** @param {AuthoredRetainedPair[]} chain @returns {boolean} */
  refuses(chain) {
    return this.#refusedChain(chain)
  }

  /** @returns {void} */
  commit() {
    this.#syncElements()
    this.#syncTexts()
    this.#commitChildren()
    this.#finalize()
  }

  /** @param {AuthoredElement} previous @param {AuthoredElement} fresh @returns {AuthoredRetainedPair | null} */
  static componentPair(previous, fresh) {
    const pair = retainedPair(previous)
    return AuthoredRootUpdate.#requestedPair(pair, fresh)
  }

  /** @param {AuthoredElement} host @returns {AuthoredElement} */
  static componentHost(host) {
    const pair = retainedPair(host)
    return AuthoredRootUpdate.#actualComponentHost(pair, host)
  }

  /** @param {AuthoredRetainedPair | null} pair @param {AuthoredElement} host @returns {AuthoredElement} */
  static #actualComponentHost(pair, host) {
    if (!pair || pair.previous.node.k !== AuthoredRootUpdate.#componentKind ||
      pair.previous.path !== Elements.records.get(pair.fresh)?.path) return host
    return pair.fresh
  }

  /** @param {AuthoredRetainedPair | null} pair @param {AuthoredElement} fresh @returns {AuthoredRetainedPair | null} */
  static #requestedPair(pair, fresh) {
    const requested = pair?.fresh === fresh
    if (requested) return pair
    return null
  }

  /** @param {AuthoredParent} parent @returns {AuthoredChild[]} */
  static #recordedChildren(parent) {
    return Elements.records.get(parent)?.children ?? [...parent.childNodes]
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild[]} desired @param {AuthoredChild[]} previous @returns {void} */
  #planChildren(parent, desired, previous) {
    this.#children.set(parent, desired)
    const index = this.#childIndex(previous)
    this.#planNodes(parent, desired, index)
  }

  /** @param {AuthoredChild[]} children @returns {Map<string | undefined, AuthoredChild>} */
  #childIndex(children) {
    return new Map(children.map(child => [this.#childPath(child), child]))
  }

  /** @param {AuthoredChild} child @returns {string | undefined} */
  #childPath(child) {
    const record = Elements.records.get(child) ?? Elements.textRecords.get(child)
    return record?.path
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild[]} children @param {Map<string | undefined, AuthoredChild>} index @returns {void} */
  #planNodes(parent, children, index) {
    for (const fresh of children) this.#planNode(parent, fresh, index)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {Map<string | undefined, AuthoredChild>} index @returns {void} */
  #planNode(parent, fresh, index) {
    const previous = index.get(this.#childPath(fresh))
    const pair = retainedPair(previous)
    this.#prepareElement(parent, fresh, pair)
    this.#prepareText(parent, fresh, previous)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {AuthoredRetainedPair | null} pair @returns {void} */
  #prepareElement(parent, fresh, pair) {
    if (pair) this.#preparePair(parent, fresh, pair)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {AuthoredChild | undefined} previous @returns {void} */
  #prepareText(parent, fresh, previous) {
    const inputs = this.#textInputs(parent, fresh, previous)
    this.#recordTextPair(inputs)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {AuthoredChild | undefined} previous @returns {AuthoredTextInputs} */
  #textInputs(parent, fresh, previous) {
    return { parent, old: previous, fresh,
      previous: Elements.textRecords.get(previous ?? fresh), next: Elements.textRecords.get(fresh) }
  }

  /** @param {AuthoredTextInputs} inputs @returns {void} */
  #recordTextPair(inputs) {
    const { old, fresh, previous, next, parent } = inputs
    if (!(old instanceof Text) || !(fresh instanceof Text) || !previous || !next ||
      old.parentNode !== parent || previous.node !== next.node || previous.owner !== next.owner ||
      previous.path !== next.path || !sameLocals(previous.locals, next.locals) || old.data !== previous.value) return
    this.#retained.set(fresh, old)
    this.#textPairs.push({ old, fresh, next })
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {AuthoredRetainedPair} pair @returns {void} */
  #preparePair(parent, fresh, pair) {
    const identity = this.#samePair(parent, fresh, pair)
    const layouts = [this.#nativeLayout(pair), ...this.#slotLayouts(pair)]
    const layout = layouts.find(candidate => [identity, Boolean(candidate)].every(Boolean))
    this.#recordPair(pair, layout)
    this.#planLayouts(layout)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} fresh @param {AuthoredRetainedPair} pair @returns {boolean} */
  #samePair(parent, fresh, pair) {
    return [pair.fresh === fresh, pair.old.parentNode === parent,
      pair.previous.path === Elements.records.get(fresh)?.path].every(Boolean)
  }

  /** @param {AuthoredRetainedPair} pair @returns {AuthoredChildLayout[] | null} */
  #nativeLayout(pair) {
    if (!this.#nativeSupported(pair)) return null
    return [{ parent: pair.old, previous: pair.previous.children,
      desired: Elements.records.get(pair.fresh).children }]
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #nativeSupported(pair) {
    return [this.#nativeName(pair), this.#originalDirect(pair),
      !changesObservedAttribute(pair)].every(Boolean)
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #nativeName(pair) {
    const html = pair.old.namespaceURI === AuthoredRootUpdate.#htmlNamespace
    return [pair.old.namespaceURI === AuthoredRootUpdate.#svgNamespace,
      [html, !pair.old.localName.includes('-'),
        !pair.old.matches(AuthoredRootUpdate.#editingControls)].every(Boolean),
      [html, pair.old.localName === AuthoredRootUpdate.#componentTag,
        pair.previous.node.k === AuthoredRootUpdate.#componentKind].every(Boolean)].includes(true)
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #originalDirect(pair) {
    return this.#sameChildren([...pair.old.childNodes], pair.previous.children)
  }

  /** @param {AuthoredChild[]} current @param {AuthoredChild[]} previous @returns {boolean} */
  #sameChildren(current, previous) {
    return [current.length === previous.length,
      previous.every((child, index) => child === current[index])].every(Boolean)
  }

  /** @param {AuthoredRetainedPair} pair @returns {AuthoredChildLayout[][]} */
  #slotLayouts(pair) {
    const candidates = [pair].filter(candidate => this.#slotHost(candidate))
    return this.#distributedLayouts(candidates)
  }

  /** @param {AuthoredRetainedPair[]} candidates @returns {AuthoredChildLayout[][]} */
  #distributedLayouts(candidates) {
    return candidates.flatMap(pair => this.#groups(pair))
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #slotHost(pair) {
    return [pair.old.namespaceURI === AuthoredRootUpdate.#htmlNamespace,
      pair.old.localName.includes('-'),
      pair.previous.node.k !== AuthoredRootUpdate.#componentKind,
      !changesObservedAttribute(pair)].every(Boolean)
  }

  /** @param {AuthoredRetainedPair} pair @returns {AuthoredChildLayout[][]} */
  #groups(pair) {
    const slots = ownedSlots(pair.old)
    const groups = this.#slotIndex(slots)
    const previousKnown = this.#fillGroups(pair.previous.children, 'previous', groups)
    const desiredKnown = this.#fillGroups(Elements.records.get(pair.fresh).children, 'desired', groups)
    const valid = [slots.length > 0, slots.length === groups.size, previousKnown,
      desiredKnown, this.#originalSlots(groups), this.#sameAssignments(pair)].every(Boolean)
    const layout = this.#slotChildren(groups)
    return [layout].filter(() => valid)
  }

  /** @param {HTMLSlotElement[]} slots @returns {Map<string, AuthoredSlotGroup>} */
  #slotIndex(slots) {
    return new Map(slots.map(slot => [slot.getAttribute('name') ?? AuthoredRootUpdate.#defaultSlot,
      { slot, previous: [], desired: [] }]))
  }

  /** @param {AuthoredChild[]} children @param {'previous' | 'desired'} key @param {Map<string, AuthoredSlotGroup>} groups @returns {boolean} */
  #fillGroups(children, key, groups) {
    const known = []
    for (const child of children) known.push(this.#fillGroup(child, key, groups))
    return known.every(Boolean)
  }

  /** @param {AuthoredChild} child @param {'previous' | 'desired'} key @param {Map<string, AuthoredSlotGroup>} groups @returns {boolean} */
  #fillGroup(child, key, groups) {
    const group = groups.get(this.#slotName(child))
    if (!group) return false
    group[key].push(child)
    return true
  }

  /** @param {AuthoredChild | undefined} child @returns {string} */
  #slotName(child) {
    return Elements.records.get(child)?.attributes.get('slot') ?? AuthoredRootUpdate.#defaultSlot
  }

  /** @param {Map<string, AuthoredSlotGroup>} groups @returns {boolean} */
  #originalSlots(groups) {
    return [...groups.values()].every(group => this.#originalSlot(group))
  }

  /** @param {AuthoredSlotGroup} group @returns {boolean} */
  #originalSlot(group) {
    return [this.#sameChildren([...group.slot.childNodes], group.previous),
      this.#currentAssignments(group.previous)].every(Boolean)
  }

  /** @param {AuthoredChild[]} children @returns {boolean} */
  #currentAssignments(children) {
    return children.every(child => this.#slotName(child) ===
      (child.getAttribute?.('slot') ?? AuthoredRootUpdate.#defaultSlot))
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #sameAssignments(pair) {
    const next = this.#childIndex(Elements.records.get(pair.fresh).children)
    return pair.previous.children.every(child => this.#sameAssignment(child, next))
  }

  /** @param {AuthoredChild} child @param {Map<string | undefined, AuthoredChild>} next @returns {boolean} */
  #sameAssignment(child, next) {
    const fresh = next.get(Elements.records.get(child)?.path)
    return [!fresh, this.#slotName(child) === this.#slotName(fresh)].includes(true)
  }

  /** @param {Map<string, AuthoredSlotGroup>} groups @returns {AuthoredChildLayout[]} */
  #slotChildren(groups) {
    return [...groups.values()].map(group => ({ parent: group.slot,
      previous: group.previous, desired: group.desired }))
  }

  /** @param {AuthoredRetainedPair} pair @param {AuthoredChildLayout[] | undefined} layout @returns {void} */
  #recordPair(pair, layout) {
    if (!layout) {
      this.#refused.add(pair.old)
      return
    }
    this.#retained.set(pair.fresh, pair.old)
    this.#existing.add(pair.old)
    this.#pairs.push({ ...pair, distributed: this.#distributedHost(pair) })
  }

  /** @param {AuthoredChildLayout[]} [layout] @returns {void} */
  #planLayouts(layout = []) {
    for (const group of layout) this.#planChildren(group.parent, group.desired, group.previous)
  }

  /** @param {AuthoredRetainedPair[]} chain @returns {boolean} */
  #refusedChain(chain) {
    return chain.some(pair => [this.#refused.has(pair.old), pair.old.localName.includes('-')].every(Boolean))
  }

  /** @returns {void} */
  #syncElements() {
    for (const pair of this.#pairs) this.#syncElement(pair)
  }

  /** @returns {void} */
  #syncTexts() {
    for (const pair of this.#textPairs) this.#syncText(pair)
  }

  /** @param {AuthoredTextPair} pair @returns {void} */
  #syncText(pair) {
    if (pair.old.data !== pair.fresh.data) pair.old.data = pair.fresh.data
  }

  /** @param {AuthoredPlannedPair} pair @returns {void} */
  #syncElement(pair) {
    const next = Elements.records.get(pair.fresh)
    this.#syncAttributes(pair, next)
    syncControlValue(pair.old, pair.fresh, next, this.#active, this.#editingAtStart)
    syncControlFlags(pair.old, pair.fresh, pair.previous, next)
    transferEventListeners(pair.old, pair.fresh)
    this.#componentState(pair.old, pair.fresh)
  }

  /** @param {AuthoredRetainedPair} pair @returns {boolean} */
  #distributedHost(pair) {
    return [pair.old.namespaceURI === AuthoredRootUpdate.#htmlNamespace,
      pair.old.localName.includes('-'),
      pair.previous.node.k !== AuthoredRootUpdate.#componentKind].every(Boolean)
  }

  /** @param {AuthoredPlannedPair} pair @param {AuthoredRecord} next @returns {void} */
  #syncAttributes(pair, next) {
    const previous = this.#authoredAttributes(pair, pair.previous)
    const following = this.#authoredAttributes(pair, next)
    syncAuthoredAttributes(pair.old, previous, following)
    this.#customClass(pair, next)
  }

  /** @param {AuthoredPlannedPair} pair @param {AuthoredRecord} record @returns {AuthoredRecord} */
  #authoredAttributes(pair, record) {
    const attributes = new Map(record.attributes)
    this.#excludeCustomClass(pair, attributes)
    return { ...record, attributes }
  }

  /** @param {AuthoredPlannedPair} pair @param {Map<string, string>} attributes @returns {void} */
  #excludeCustomClass(pair, attributes) {
    if (pair.distributed) attributes.delete(AuthoredRootUpdate.#classAttribute)
  }

  /** @param {AuthoredPlannedPair} pair @param {AuthoredRecord} next @returns {void} */
  #customClass(pair, next) {
    const previous = pair.previous.attributes.get(AuthoredRootUpdate.#classAttribute)
    const following = next.attributes.get(AuthoredRootUpdate.#classAttribute)
    if ([pair.distributed, previous !== following].includes(false)) return
    pair.old.classList.remove(...this.#classTokens(previous))
    pair.old.classList.add(...this.#classTokens(following))
  }

  /** @param {string} [value] @returns {string[]} */
  #classTokens(value = AuthoredRootUpdate.#defaultSlot) {
    return value.trim().split(/\s+/).filter(Boolean)
  }

  /** @param {AuthoredElement} element @param {AuthoredElement} fresh @returns {void} */
  #componentState(element, fresh) {
    if (fresh.tachyonComponent) element.tachyonComponent = fresh.tachyonComponent
  }

  /** @returns {void} */
  #commitChildren() {
    for (const [parent, desired] of this.#children) this.#commitGroup(parent, desired)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild[]} desired @returns {void} */
  #commitGroup(parent, desired) {
    const children = desired.map(child => this.#resolve(child))
    this.#removeUnwanted(parent, new Set(children))
    this.#insertChildren(parent, children)
  }

  /** @param {AuthoredParent} parent @param {Set<AuthoredChild>} desired @returns {void} */
  #removeUnwanted(parent, desired) {
    for (const child of [...parent.childNodes]) this.#removeChild(child, desired)
  }

  /** @param {AuthoredChild} child @param {Set<AuthoredChild>} desired @returns {void} */
  #removeChild(child, desired) {
    if (!desired.has(child)) child.remove()
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild[]} children @returns {void} */
  #insertChildren(parent, children) {
    let cursor = parent.firstChild
    for (const child of children) cursor = this.#placeChild(parent, child, cursor)
  }

  /** @param {AuthoredParent} parent @param {AuthoredChild} child @param {ChildNode | null} cursor @returns {ChildNode | null} */
  #placeChild(parent, child, cursor) {
    if (child !== cursor) parent.insertBefore(child, cursor)
    return child.nextSibling
  }

  /** @param {AuthoredChild} child @returns {AuthoredChild} */
  #resolve(child) {
    return this.#retained.get(child) ?? child
  }

  /** @returns {void} */
  #finalize() {
    this.#recordElements()
    this.#recordTexts()
    this.#renderElements()
    Rendering.mounts = Rendering.mounts.map(([host, ...arguments_]) => [this.#resolve(host), ...arguments_])
  }

  /** @returns {void} */
  #recordElements() {
    for (const pair of this.#pairs) this.#recordElement(pair)
  }

  /** @returns {void} */
  #recordTexts() {
    for (const pair of this.#textPairs) Elements.textRecords.set(pair.old, pair.next)
  }

  /** @param {AuthoredPlannedPair} pair @returns {void} */
  #recordElement(pair) {
    const next = Elements.records.get(pair.fresh)
    Elements.records.set(pair.old, { ...next,
      children: next.children.map(child => this.#resolve(child)) })
  }

  /** @returns {void} */
  #renderElements() {
    for (const [path, fresh] of Rendering.elements) this.#renderElement(path, fresh)
  }

  /** @param {string} path @param {AuthoredElement} fresh @returns {void} */
  #renderElement(path, fresh) {
    const element = /** @type {AuthoredElement} */ (this.#resolve(fresh))
    const record = Elements.records.get(element)
    Rendering.elements.set(path, element)
    record.children = record.children.map(child => this.#resolve(child))
  }
}
