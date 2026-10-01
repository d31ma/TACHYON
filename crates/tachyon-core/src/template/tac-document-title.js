class TacDocumentTitle {
  static mostBytes = 1024

  static owned(signal) {
    const page = document
    return Object.freeze({ set: (...arguments_) => TacDocumentTitle.#write(page, signal, arguments_) })
  }

  static host(...arguments_) {
    const title = TacDocumentTitle.#argument(arguments_)
    TacDocumentTitle.#validate(title)
    document.title = title
    return null
  }

  static #write(page, signal, arguments_) {
    const title = TacDocumentTitle.#argument(arguments_)
    TacDocumentTitle.#owner(page, signal)
    TacDocumentTitle.#validate(title)
    page.title = title
  }

  static #owner(page, signal) {
    if (!signal || signal.aborted || page !== document) throw new Error('DocumentTitle owner is disposed or unavailable.')
  }

  static #validate(title) {
    TacDocumentTitle.#type(title)
    TacDocumentTitle.#unicode(title)
    TacDocumentTitle.#size(title)
  }

  static #type(title) {
    TacDocumentTitle.#string(title)
    TacDocumentTitle.#nul(title)
  }

  static #argument(arguments_) {
    if (arguments_.length !== 1) throw new TypeError('DocumentTitle.set needs exactly one title.')
    return arguments_[0]
  }

  static #string(title) {
    if (typeof title !== 'string') throw new TypeError('DocumentTitle needs a string.')
  }

  static #nul(title) {
    if (title.includes('\u0000')) throw new TypeError('DocumentTitle rejects NUL.')
  }

  static #unicode(title) {
    for (const character of title) TacDocumentTitle.#scalar(character.codePointAt(0))
  }

  static #scalar(point) {
    if (point >= 0xd800 && point <= 0xdfff) throw new TypeError('DocumentTitle needs valid Unicode.')
  }

  static #size(title) {
    if (new TextEncoder().encode(title).byteLength > TacDocumentTitle.mostBytes) throw new RangeError('DocumentTitle exceeds 1024 UTF-8 bytes.')
  }
}
