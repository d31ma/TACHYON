@Setup
export default class PageProgramSetup {
  static configure() {
    const state = { identity: crypto.randomUUID(), count: 0 }
    return {
      http: {
        pageprogram: {
          exchange(request) {
            if (request.method === 'POST' && request.path.endsWith('/increment')) state.count += 1
            return { identity: state.identity, count: state.count }
          },
        },
      },
    }
  }
}
