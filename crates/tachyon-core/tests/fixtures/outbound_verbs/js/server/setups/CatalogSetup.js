@Setup
export default class CatalogSetup {
  static configure() {
    return { http: { catalog: { exchange: CatalogSetup.echo } } };
  }

  static echo(request) {
    return { method: request.method, path: request.path, query: request.query, headers: request.headers, body: request.body ?? null };
  }
}
