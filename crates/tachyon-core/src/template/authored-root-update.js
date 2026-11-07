class AuthoredRootUpdate {
  static _htmlNamespace = 'http://www.w3.org/1999/xhtml'
  static _svgNamespace = 'http://www.w3.org/2000/svg'
  static _componentTag = 'tachyon-component'
  static _componentKind = 'component'
  static _editingControls = 'textarea, [contenteditable]'
  static _defaultSlot = ''
  _container
  _source
  _active
  _editingAtStart
  _retained = new Map()
  _refused = new Set()
  _pairs = []
  _children = new Map()
  _texts = []

  constructor(container, source, active, editingAtStart) {
    this._container = container
    this._source = source
    this._active = active
    this._editingAtStart = editingAtStart
  }
  prepare() {
    const previous = AuthoredRootUpdate._recordedChildren(this._container)
    const desired = AuthoredRootUpdate._recordedChildren(this._source)
    this._planChildren({ parent: this._container, previous, desired })
    return this
  }
  retains(element) {
    for (const old of this._retained.values()) {
      if (old === element) return true
    }
    return false
  }
  refuses(chain) {
    return chain.some((pair) => this._refused.has(pair.old) && AuthoredRootUpdate._isCustom(pair.old))
  }
  commit() {
    const selections = this._pairs.filter((pair) => pair.old instanceof HTMLSelectElement).map((pair) => this._selection(pair))
    for (const pair of this._pairs) this._syncElement(pair)
    for (const text of this._texts) this._syncText(text)
    for (const [parent, desired] of this._children) this._commitGroup(parent, desired)
    for (const selection of selections) AuthoredRootUpdate._restoreSelection(selection)
    this._recordElements()
  }
  _selection(pair) {
    const rendered = [...pair.fresh.selectedOptions].map((option) => this._resolve(option))
    const kept = this._bindsSelection(pair) ? rendered : [...pair.old.selectedOptions]
    return { select: pair.old, kept, rendered }
  }
  _bindsSelection(pair) {
    if (changedDuringRender(pair.old, this._active, this._editingAtStart)) return false
    return Elements.records.get(pair.fresh).attributes.has('value') || pair.fresh.querySelector('option[selected]') !== null
  }
  static _restoreSelection({ select, kept, rendered }) {
    const options = [...select.options]
    const surviving = kept.filter((option) => options.includes(option))
    const chosen = new Set(surviving.length ? surviving : rendered)
    for (const option of options) {
      if (option.selected !== chosen.has(option)) option.selected = chosen.has(option)
    }
  }
  static _recordedChildren(parent) {
    const record = Elements.records.get(parent)
    if (record) return record.children
    return [...parent.childNodes]
  }
  static _isCustom(element) {
    return element.localName.includes('-')
  }
  static _slotName(node) {
    const record = Elements.records.get(node)
    return record?.attributes.get('slot') ?? AuthoredRootUpdate._defaultSlot
  }
  static _sameChildren(current, previous) {
    if (current.length !== previous.length) return false
    return previous.every((child, index) => child === current[index])
  }
  static _byPath(children) {
    const index = new Map()
    for (const child of children) {
      const record = Elements.records.get(child)
      if (record) index.set(record.path, child)
    }
    return index
  }
  _planChildren(group) {
    this._children.set(group.parent, group.desired)
    const previousByPath = AuthoredRootUpdate._byPath(group.previous)
    for (let index = 0; index < group.desired.length; index += 1) {
      const fresh = group.desired[index]
      if (fresh.nodeType === Node.TEXT_NODE) {
        this._planText(group.parent, group.previous[index], fresh)
        continue
      }
      const record = Elements.records.get(fresh)
      if (!record) continue
      const old = previousByPath.get(record.path)
      if (!old) continue
      const pair = retainedPair(old)
      if (pair) this._planPair(group.parent, fresh, pair)
    }
  }
  _planText(parent, old, fresh) {
    if (old?.nodeType !== Node.TEXT_NODE) return
    if (old.parentNode !== parent) return
    this._retained.set(fresh, old)
    this._texts.push({ old, fresh })
  }
  _planPair(parent, fresh, pair) {
    const layout = this._samePlace(parent, fresh, pair) ? this._layout(pair) : null
    if (!layout) {
      this._refused.add(pair.old)
      return
    }
    this._retained.set(pair.fresh, pair.old)
    this._pairs.push(pair)
    for (const group of layout) this._planChildren(group)
  }
  _samePlace(parent, fresh, pair) {
    if (pair.fresh !== fresh) return false
    return pair.old.parentNode === parent
  }
  _layout(pair) {
    if (changesObservedAttribute(pair)) return null
    if (!this._retainable(pair)) return null
    const slots = this._lightSlots(pair)
    if (slots.length) return this._slotLayout(pair, slots)
    return this._directLayout(pair)
  }
  _retainable(pair) {
    const element = pair.old
    if (element.namespaceURI === AuthoredRootUpdate._svgNamespace) return true
    if (element.namespaceURI !== AuthoredRootUpdate._htmlNamespace) return false
    if (element.localName === AuthoredRootUpdate._componentTag) {
      return pair.previous.node.k === AuthoredRootUpdate._componentKind
    }
    return !element.matches(AuthoredRootUpdate._editingControls)
  }
  _lightSlots(pair) {
    if (pair.previous.node.k === AuthoredRootUpdate._componentKind) return []
    if (!AuthoredRootUpdate._isCustom(pair.old)) return []
    return ownedSlots(pair.old)
  }
  _directLayout(pair) {
    const current = [...pair.old.childNodes]
    if (!AuthoredRootUpdate._sameChildren(current, pair.previous.children)) return null
    const desired = Elements.records.get(pair.fresh).children
    return [{ parent: pair.old, previous: pair.previous.children, desired }]
  }
  _slotLayout(pair, slots) {
    const groups = new Map()
    for (const slot of slots) {
      const name = slot.getAttribute('name') ?? AuthoredRootUpdate._defaultSlot
      groups.set(name, { parent: slot, previous: [], desired: [] })
    }
    if (groups.size !== slots.length) return null
    const desired = Elements.records.get(pair.fresh).children
    if (!this._assign(pair.previous.children, groups, 'previous')) return null
    if (!this._assign(desired, groups, 'desired')) return null
    if (!this._slotsUnchanged(groups)) return null
    if (!this._sameAssignments(pair.previous.children, desired)) return null
    return [...groups.values()]
  }

  _assign(children, groups, side) {
    for (const child of children) {
      const group = groups.get(AuthoredRootUpdate._slotName(child))
      if (!group) return false
      group[side].push(child)
    }
    return true
  }
  _slotsUnchanged(groups) {
    for (const group of groups.values()) {
      const current = [...group.parent.childNodes]
      if (!AuthoredRootUpdate._sameChildren(current, group.previous)) return false
      if (!group.previous.every((child) => this._keepsSlotAttribute(child))) return false
    }
    return true
  }
  _keepsSlotAttribute(child) {
    const current = child.getAttribute?.('slot') ?? AuthoredRootUpdate._defaultSlot
    return AuthoredRootUpdate._slotName(child) === current
  }
  _sameAssignments(previous, desired) {
    const desiredByPath = AuthoredRootUpdate._byPath(desired)
    for (const child of previous) {
      const path = Elements.records.get(child)?.path
      const fresh = desiredByPath.get(path)
      if (fresh && AuthoredRootUpdate._slotName(child) !== AuthoredRootUpdate._slotName(fresh)) return false
    }
    return true
  }
  _syncElement(pair) {
    const next = Elements.records.get(pair.fresh)
    syncAuthoredAttributes(pair.old, pair.previous, next)
    syncControlValue(pair.old, pair.fresh, next, this._active, this._editingAtStart)
    syncControlFlags(pair.old, pair.fresh, pair.previous, next)
    transferEventListeners(pair.old, pair.fresh)
    if (pair.fresh.tachyonComponent) pair.old.tachyonComponent = pair.fresh.tachyonComponent
  }
  _syncText(text) {
    if (text.old.data !== text.fresh.data) text.old.data = text.fresh.data
  }
  _commitGroup(parent, desired) {
    const children = desired.map((child) => this._resolve(child))
    const wanted = new Set(children)
    for (const child of [...parent.childNodes]) {
      if (!wanted.has(child)) child.remove()
    }
    let cursor = parent.firstChild
    for (const child of children) {
      if (child !== cursor) parent.insertBefore(child, cursor)
      cursor = child.nextSibling
    }
  }
  _resolve(child) {
    return this._retained.get(child) ?? child
  }
  _recordElements() {
    for (const pair of this._pairs) {
      const next = Elements.records.get(pair.fresh)
      Elements.records.set(pair.old, { ...next, children: next.children.map((child) => this._resolve(child)) })
    }
    for (const [path, fresh] of Rendering.elements) {
      const element = this._resolve(fresh)
      Rendering.elements.set(path, element)
      const record = Elements.records.get(element)
      record.children = record.children.map((child) => this._resolve(child))
    }
    Rendering.mounts = Rendering.mounts.map(([host, ...rest]) => [this._resolve(host), ...rest])
  }
}
