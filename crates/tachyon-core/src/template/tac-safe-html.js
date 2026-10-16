class TacSafeHtml {
  static mostCharacters = 1048576
  static mostNodes = 20000
  static droppedTags = new Set(['script', 'style', 'iframe', 'frame', 'frameset', 'noframes', 'object', 'embed', 'applet', 'param',
    'template', 'noscript', 'noembed', 'svg', 'math', 'form', 'input', 'button', 'select', 'option', 'optgroup', 'textarea', 'datalist',
    'link', 'meta', 'base', 'title', 'head', 'video', 'audio', 'source', 'track', 'picture', 'canvas', 'dialog', 'slot', 'portal',
    'xmp', 'plaintext', 'listing', 'marquee', 'frame', 'image'])
  static keptTags = new Set(['a', 'abbr', 'address', 'b', 'bdi', 'bdo', 'blockquote', 'br', 'caption', 'center', 'cite', 'code', 'col',
    'colgroup', 'dd', 'del', 'details', 'dfn', 'div', 'dl', 'dt', 'em', 'figcaption', 'figure', 'font', 'h1', 'h2', 'h3', 'h4', 'h5',
    'h6', 'hr', 'i', 'img', 'ins', 'kbd', 'li', 'mark', 'ol', 'p', 'pre', 'q', 's', 'samp', 'small', 'span', 'strike', 'strong', 'sub',
    'summary', 'sup', 'table', 'tbody', 'td', 'tfoot', 'th', 'thead', 'time', 'tr', 'tt', 'u', 'ul', 'var', 'wbr'])
  static keptAttributes = new Set(['abbr', 'align', 'alt', 'bgcolor', 'border', 'cellpadding', 'cellspacing', 'cite', 'color', 'colspan',
    'datetime', 'dir', 'face', 'headers', 'height', 'lang', 'reversed', 'rowspan', 'scope', 'size', 'span', 'start', 'title', 'type',
    'valign', 'width'])
  static urlAttributes = new Map([['a', 'href'], ['img', 'src']])
  static linkProtocols = ['http:', 'https:', 'mailto:', 'tel:']
  static imageProtocols = ['http:', 'https:']
  static dataImage = /^data:image\/(?:png|gif|jpeg|webp);base64,[a-z0-9+/=\s]*$/i
  static styleProperties = /^(?:color|background-color|font(?:-[a-z]+)?|text-(?:align|decoration|indent|transform)|line-height|letter-spacing|word-spacing|white-space|vertical-align|(?:margin|padding)(?:-(?:top|right|bottom|left))?|border(?:-(?:top|right|bottom|left))?(?:-(?:color|style|width))?|border-(?:collapse|spacing|radius)|width|height|max-width|min-width|max-height|min-height|list-style-type|direction|display|table-layout)$/
  static unsafeStyleValue = /url\s*\(|expression\s*\(|javascript:|@import|\\|<|>/i

  /** @param {unknown} html @returns {DocumentFragment} */
  static fragment(html) {
    const text = String(html ?? '')
    if (text.length > TacSafeHtml.mostCharacters) throw new RangeError('Tac safe-html content exceeds 1 MiB.')
    const parsed = new DOMParser().parseFromString(text, 'text/html')
    const fragment = document.createDocumentFragment()
    const budget = { nodes: 0 }
    for (const child of [...parsed.body.childNodes]) TacSafeHtml.#copy(child, fragment, budget)
    return fragment
  }

  /** @param {Node} source @param {Node} target @param {{nodes: number}} budget */
  static #copy(source, target, budget) {
    budget.nodes += 1
    if (budget.nodes > TacSafeHtml.mostNodes) throw new RangeError('Tac safe-html content exceeds 20,000 nodes.')
    if (source.nodeType === Node.TEXT_NODE) {
      target.appendChild(document.createTextNode(source.nodeValue ?? ''))
      return
    }
    if (source.nodeType !== Node.ELEMENT_NODE || source.namespaceURI !== 'http://www.w3.org/1999/xhtml') return
    const tag = source.localName
    if (TacSafeHtml.droppedTags.has(tag)) return
    const kept = TacSafeHtml.keptTags.has(tag)
    const holder = kept ? TacSafeHtml.#element(source, tag) : target
    for (const child of [...source.childNodes]) TacSafeHtml.#copy(child, holder, budget)
    if (kept) target.appendChild(holder)
  }

  /** @param {Element} source @param {string} tag @returns {Element} */
  static #element(source, tag) {
    const element = document.createElement(tag)
    for (const attribute of [...source.attributes]) {
      if (TacSafeHtml.keptAttributes.has(attribute.name)) element.setAttribute(attribute.name, attribute.value)
    }
    TacSafeHtml.#copyStyle(source, element)
    TacSafeHtml.#copyUrl(source, element, tag)
    return element
  }

  /** @param {Element} source @param {Element} element */
  static #copyStyle(source, element) {
    const style = source.getAttribute('style')
    if (!style) return
    const probe = document.createElement('div')
    probe.setAttribute('style', style)
    const declarations = []
    for (const name of [...probe.style]) {
      const value = probe.style.getPropertyValue(name)
      if (TacSafeHtml.styleProperties.test(name) && !TacSafeHtml.unsafeStyleValue.test(value)) declarations.push(`${name}: ${value}`)
    }
    if (declarations.length) element.setAttribute('style', declarations.join('; '))
  }

  /** @param {Element} source @param {Element} element @param {string} tag */
  static #copyUrl(source, element, tag) {
    const name = TacSafeHtml.urlAttributes.get(tag)
    const value = name ? source.getAttribute(name) : null
    if (!name || value === null) return
    const safe = tag === 'img' ? TacSafeHtml.#imageUrl(value) : TacSafeHtml.#linkUrl(value)
    if (safe === null) return
    element.setAttribute(name, safe)
    if (tag === 'a') {
      element.setAttribute('rel', 'noopener noreferrer nofollow')
      element.setAttribute('target', '_blank')
    }
  }

  /** @param {string} value @returns {string | null} */
  static #linkUrl(value) {
    const url = TacSafeHtml.#url(value)
    return url && TacSafeHtml.linkProtocols.includes(url.protocol) ? url.href : null
  }

  /** @param {string} value @returns {string | null} */
  static #imageUrl(value) {
    const trimmed = value.trim()
    if (TacSafeHtml.dataImage.test(trimmed)) return trimmed
    const url = TacSafeHtml.#url(trimmed)
    return url && TacSafeHtml.imageProtocols.includes(url.protocol) ? url.href : null
  }

  /** @param {string} value @returns {URL | null} */
  static #url(value) {
    try { return new URL(value.replace(/[\u0000-\u001f\u007f\s]+/g, ''), location.href) } catch { return null }
  }
}
