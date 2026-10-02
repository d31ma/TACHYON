@RequestRoute("/api/cached-probe")
@Public
export default class CachedProbeController {
  static CACHE_CONTROL = 'cache-control'

  /** @returns {Response} */
  @GetRoute
  getProbe() {
    return Response.json({ message: 'actual public Yon answer' }, { headers: { [CachedProbeController.CACHE_CONTROL]: 'public, max-age=60' } })
  }

  /** @returns {Response} */
  @PostRoute
  postProbe() {
    return Response.json({ message: 'actual Yon write' }, { headers: { [CachedProbeController.CACHE_CONTROL]: 'no-store' } })
  }
}
