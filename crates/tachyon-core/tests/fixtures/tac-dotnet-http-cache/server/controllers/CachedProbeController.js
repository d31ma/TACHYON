@RequestRoute("/api/cached-probe")
@Public
export default class CachedProbeController {
  static _cacheControl = 'cache-control'

  /** @returns {Response} */
  @GetRoute
  getProbe() {
    return Response.json({ message: 'actual public Yon answer' }, { headers: { [CachedProbeController._cacheControl]: 'public, max-age=60' } })
  }

  /** @returns {Response} */
  @PostRoute
  postProbe() {
    return Response.json({ message: 'actual Yon write' }, { headers: { [CachedProbeController._cacheControl]: 'no-store' } })
  }
}
