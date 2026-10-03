@RequestRoute('/session')
@Public
export class SessionController {
  @PostRoute
  post() {
    const headers = {
      'content-type': 'application/json',
      'set-cookie': 'session=app-only; Path=/; HttpOnly; SameSite=Lax',
    }
    return new Response(JSON.stringify({ saved: true }), { headers })
  }

  @GetRoute
  get(request) {
    return { cookie: String(request.headers.get('cookie')) }
  }
}
