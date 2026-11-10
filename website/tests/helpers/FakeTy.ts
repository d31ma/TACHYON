class FakeTy {
  static readonly SLOW_MILLISECONDS = 3000
  static readonly COLD_MILLISECONDS = 1500
  static readonly BIG_ANSWER_BYTES = 100 * 1024
  static cold = true

  static start(): void {
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: request => FakeTy.answer(request) })
    console.log(`Tachyon production server ready at http://127.0.0.1:${server.port}/`)
    process.on('SIGTERM', () => process.exit(0))
  }

  static async answer(request: Request): Promise<Response> {
    const url = new URL(request.url)
    if (url.pathname === '/slow') await Bun.sleep(FakeTy.SLOW_MILLISECONDS)
    if (url.pathname === '/cold' && FakeTy.cold) await Bun.sleep(FakeTy.COLD_MILLISECONDS)
    FakeTy.cold = false
    if (url.pathname === '/big') return new Response('x'.repeat(FakeTy.BIG_ANSWER_BYTES), { headers: { 'content-type': 'text/plain' } })
    return Response.json({
      pid: process.pid,
      method: request.method,
      path: url.pathname,
      query: Object.fromEntries(url.searchParams),
      host: request.headers.get('host'),
      forwardedFor: request.headers.get('x-forwarded-for'),
      body: await request.text(),
      environment: Object.keys(process.env).sort(),
      project: process.argv[3],
    })
  }
}

FakeTy.start()
