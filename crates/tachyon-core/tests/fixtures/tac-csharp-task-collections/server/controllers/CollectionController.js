@RequestRoute('/api/collections')
@Public
export default class CollectionController {
  /** @type {string} */
  static _cacheControl = 'cache-control'
  /** @type {string} */
  static _noStore = 'no-store'
  /** @type {string} */
  static _first = 'first'
  /** @type {number} */
  static _delayMilliseconds = 500

  /** @returns {Response} */
  @GetRoute
  getCollection() {
    return CollectionController._response({ items: [{ message: CollectionController._first }, { message: 'second' }] })
  }

  /** @returns {Response} */
  @GetRoute('/empty')
  getEmpty() {
    return CollectionController._response({ items: [] })
  }

  /** @returns {Response} */
  @GetRoute('/nullable')
  getNullable() {
    return CollectionController._response({ items: [], optionalItems: [null, { message: 'nullable item' }] })
  }

  /** @returns {Response} */
  @GetRoute('/null-list')
  getNullList() {
    return CollectionController._response({ items: [], optionalItems: null })
  }

  /** @returns {Response} */
  @GetRoute('/nullable-leaf')
  getNullableLeaf() {
    return CollectionController._response({ items: [{ message: 'absent note' }, { message: 'null note', note: null }, { message: 'text note', note: 'present' }] })
  }

  /** @returns {Response} */
  @GetRoute('/wrong-nullable-leaf')
  getWrongNullableLeaf() {
    return CollectionController._response({ items: [{ message: 'valid message', note: false }] })
  }

  /** @returns {Response} */
  @GetRoute('/missing')
  getMissing() {
    return CollectionController._response({})
  }

  /** @returns {Response} */
  @GetRoute('/wrong-array')
  getWrongArray() {
    return CollectionController._response({ items: {} })
  }

  /** @returns {Response} */
  @GetRoute('/missing-member')
  getMissingMember() {
    return CollectionController._response({ items: [{}] })
  }

  /** @returns {Response} */
  @GetRoute('/wrong-member')
  getWrongMember() {
    return CollectionController._response({ items: [{ message: 7 }] })
  }

  /** @returns {Response} */
  @GetRoute('/extra-member')
  getExtraMember() {
    return CollectionController._response({ items: [{ message: CollectionController._first, extra: true }] })
  }

  /** @returns {Response} */
  @GetRoute('/null-member')
  getNullMember() {
    return CollectionController._response({ items: [null] })
  }

  /** @returns {Promise<Response>} */
  @GetRoute('/slow')
  async getSlow() {
    await CollectionController._delay()
    return CollectionController._response({ items: [{ message: 'late collection answer' }] })
  }

  /** @param {Record<string, unknown>} value @returns {Response} */
  static _response(value) {
    return Response.json(value, { headers: { [CollectionController._cacheControl]: CollectionController._noStore } })
  }

  /** @returns {Promise<void>} */
  static _delay() {
    return new Promise(resolve => setTimeout(resolve, CollectionController._delayMilliseconds))
  }
}
