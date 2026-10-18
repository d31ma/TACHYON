@RequestRoute('/api/results')
@Public
export default class ResultController {
  /** @type {string} */
  static _cacheControl = 'cache-control'
  /** @type {string} */
  static _noStore = 'no-store'

  /** @returns {Response} */
  @GetRoute('/items')
  getItems() {
    return ResultController._response([{ name: 'first item' }, { name: 'second item' }])
  }

  /** @returns {Response} */
  @GetRoute('/array')
  getArray() {
    return ResultController._response([{ name: 'array item' }])
  }

  /** @returns {Response} */
  @GetRoute('/count')
  getCount() {
    return ResultController._response(42)
  }

  /** @returns {Response} */
  @GetRoute('/label')
  getLabel() {
    return ResultController._response('Yon label')
  }

  /** @returns {Response} */
  @GetRoute('/ready')
  getReady() {
    return ResultController._response(true)
  }

  /** @returns {Response} */
  @GetRoute('/ratio')
  getRatio() {
    return ResultController._response(0.25)
  }

  /** @returns {Response} */
  @GetRoute('/numbers')
  getNumbers() {
    return ResultController._response([5, 8, 13])
  }

  /** @returns {Response} */
  @GetRoute('/shelf')
  getShelf() {
    return ResultController._response({ label: 'Yon shelf', detail: { note: 'nested note', count: 3 } })
  }

  /** @returns {Response} */
  @GetRoute('/first')
  getFirst() {
    return ResultController._response({ name: 'value task item' })
  }

  /** @returns {Response} */
  @GetRoute('/wrong-items')
  getWrongItems() {
    return ResultController._response({ name: 'not a list' })
  }

  /** @returns {Response} */
  @GetRoute('/wrong-count')
  getWrongCount() {
    return ResultController._response('forty-two')
  }

  /** @returns {Response} */
  @GetRoute('/wrong-shelf')
  getWrongShelf() {
    return ResultController._response({ label: 'broken shelf', detail: { note: 'no count' } })
  }

  /** @param {unknown} value @returns {Response} */
  static _response(value) {
    return Response.json(value, { headers: { [ResultController._cacheControl]: ResultController._noStore } })
  }
}
