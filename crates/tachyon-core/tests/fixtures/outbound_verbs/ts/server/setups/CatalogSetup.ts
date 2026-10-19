@Setup
export default class CatalogSetup {
  static configure(execution: BehaviorExecution) {
    return { http: { catalog: { exchange: CatalogSetup.echo } } };
  }

  static echo(request: { method: string, path: string, query: unknown, headers: unknown, body?: unknown }) {
    return { method: request.method, path: request.path, query: request.query, headers: request.headers, body: request.body ?? null };
  }
}
