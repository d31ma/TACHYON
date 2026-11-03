export class Middleware {
  static before(request) {
    const url = new URL(request.url)
    if (url.pathname.startsWith('/private') && request.headers.get('authorization') !== 'let-me-in') {
      return Response.json({ error: 'Unauthorized' }, { status: 401 })
    }
  }

  static after(request) {
    return new Response(null, { status: 204, headers: { 'x-served-by': 'tachyon' } })
  }
}
