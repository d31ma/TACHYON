@RequestRoute("/api/cached-probe")
@Public
export default class CachedProbeController {
  static _cacheControl = 'cache-control'
  static _noStore = 'no-store'

  /** @returns {Response} */
  @GetRoute
  getProbe() {
    return Response.json({ message: 'actual public Yon answer' }, { headers: { [CachedProbeController._cacheControl]: 'public, max-age=60' } })
  }

  /** @returns {Response} */
  @PostRoute
  postProbe() {
    return Response.json({ message: 'actual Yon write' }, { headers: { [CachedProbeController._cacheControl]: CachedProbeController._noStore } })
  }

  /** @returns {Promise<Response>} */
  @GetRoute('/slow')
  async getSlow() {
    await CachedProbeController._delay()
    return Response.json({ message: 'late Yon answer' }, { headers: { [CachedProbeController._cacheControl]: CachedProbeController._noStore } })
  }

  /** @returns {Promise<void>} */
  static _delay() {
    return new Promise(resolve => setTimeout(resolve, 500))
  }
}
