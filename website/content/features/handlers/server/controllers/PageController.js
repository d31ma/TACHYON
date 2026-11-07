@RequestRoute("/page")
@Public
class PageController {
  /** @returns {{ status: number, headers: Record<string, string>, body: string }} */
  @GetRoute
  getPage() {
    return {
      status: 202,
      headers: {
        'content-type': 'text/html; charset=utf-8',
        'x-origin': 'controller',
      },
      body: '<article><h1>Unchanged HTML</h1><p>{not-a-template}</p><if>literal</if></article>',
    }
  }
}
