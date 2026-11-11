class TacEventOptions {
  /** @type {string[]} */
  static #modifierKeys = ['shiftKey', 'ctrlKey', 'altKey', 'metaKey']
  /** @type {string} */
  static #windowTarget = 'window'
  /** @type {Map<string, {type: string, target: EventTarget, listener: (event: Event) => void, deliver?: (event: Event) => void}>} */
  static #pending = new Map()
  /** @type {Map<string, {type: string, target: EventTarget, listener: (event: Event) => void, deliver?: (event: Event) => void}>} */
  static #attached = new Map()
  /** @type {Map<Element, {element: Element}>} */
  static #holders = new Map()

  /** @param {Element} element @returns {{element: Element}} */
  static holder(element) {
    const holder = TacEventOptions.#holders.get(element) || { element }
    TacEventOptions.#holders.set(element, holder)
    return holder
  }

  /** @param {object} binding @param {(event: Event) => Promise<void>} handle @param {{element: Element}} holder @returns {(event: Event) => void} */
  static listener(binding, handle, holder) {
    return (event) => TacEventOptions.#run(binding, handle, holder, event)
  }

  /** @param {object} binding @returns {boolean} */
  static listensAbove(binding) {
    return Boolean(binding.listen || binding.outside)
  }

  /** @param {string} key @param {string} type @param {object} binding @param {(event: Event) => void} listener */
  static defer(key, type, binding, listener) {
    TacEventOptions.#pending.set(key, { type, target: TacEventOptions.#target(binding), listener })
  }

  static begin() {
    TacEventOptions.#pending = new Map()
    TacEventOptions.#holders = new Map()
  }

  static commit() {
    TacEventOptions.#attached.forEach((attached, key) => TacEventOptions.#keepOrRemove(attached, key))
    TacEventOptions.#pending.forEach((entry, key) => TacEventOptions.#attach(entry, key))
    TacEventOptions.#pending = new Map()
    TacEventOptions.#holders = new Map()
  }

  /** @param {Element} element @param {Element} fresh */
  static moved(element, fresh) {
    TacEventOptions.#point(TacEventOptions.#holders.get(fresh), element)
  }

  static #point(holder, element) {
    if (holder) holder.element = element
  }

  /** @param {object} binding @returns {EventTarget} */
  static #target(binding) {
    return binding.listen === TacEventOptions.#windowTarget ? window : document
  }

  static #run(binding, handle, holder, event) {
    if (!TacEventOptions.#matches(binding, holder, event)) return
    TacEventOptions.#prevent(binding, event)
    TacEventOptions.#stop(binding, event)
    void handle(event)
  }

  static #matches(binding, holder, event) {
    return TacEventOptions.#keyMatches(binding, event)
      && TacEventOptions.#modifierKeysMatch(binding, event)
      && TacEventOptions.#isOutside(binding, holder, event)
  }

  static #keyMatches(binding, event) {
    return !binding.key || (event.key === binding.key && !event.isComposing)
  }

  static #modifierKeysMatch(binding, event) {
    if (!Array.isArray(binding.modifierKeys)) return true
    return TacEventOptions.#modifierKeys.every((name) => Boolean(event[name]) === binding.modifierKeys.includes(name))
  }

  static #isOutside(binding, holder, event) {
    if (!binding.outside) return true
    return holder.element.isConnected && !event.composedPath().includes(holder.element)
  }

  static #prevent(binding, event) {
    if (binding.prevent) event.preventDefault()
  }

  static #stop(binding, event) {
    if (binding.stop) event.stopPropagation()
  }

  static #keepOrRemove(attached, key) {
    if (TacEventOptions.#sameListener(attached, TacEventOptions.#pending.get(key))) TacEventOptions.#keep(attached, key)
    else TacEventOptions.#remove(attached, key)
  }

  static #sameListener(attached, next) {
    return next !== undefined && next.type === attached.type && next.target === attached.target
  }

  static #keep(attached, key) {
    attached.listener = TacEventOptions.#pending.get(key).listener
    TacEventOptions.#pending.delete(key)
  }

  static #remove(attached, key) {
    attached.target.removeEventListener(attached.type, attached.deliver)
    TacEventOptions.#attached.delete(key)
  }

  static #attach(entry, key) {
    const since = performance.now()
    entry.deliver = (event) => TacEventOptions.#deliver(entry, since, event)
    entry.target.addEventListener(entry.type, entry.deliver)
    TacEventOptions.#attached.set(key, entry)
  }

  static #deliver(entry, since, event) {
    if (event.timeStamp >= since) entry.listener(event)
  }
}
