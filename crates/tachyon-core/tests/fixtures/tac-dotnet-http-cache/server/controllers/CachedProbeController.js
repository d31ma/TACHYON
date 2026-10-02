@RequestRoute("/api/cached-probe")
@Public
export default class CachedProbeController {
  /** @returns {Response} */
  @GetRoute
  getProbe() {
    return Response.json({ message: 'actual public Yon answer' }, { headers: { 'cache-control': 'public, max-age=60' } })
  }

  /** @returns {Response} */
  @PostRoute
  saveProbe() {
    return Response.json({ message: 'actual Yon write' }, { headers: { 'cache-control': 'no-store' } })
  }
}
