import type { HostAnswer } from '/models/HostAnswer.ts'

@Setup
export default class SnippetHostSetup {
  static readonly ORIGIN = 'http://snippets.localhost:8100'
  static readonly JSON = 'application/json'

  static configure() {
    return { http: { snippetHost: { exchange: (command: BehaviorHttpCommand) => SnippetHostSetup.exchange(command) } } }
  }

  static async exchange(command: BehaviorHttpCommand): Promise<HostAnswer> {
    const response = await fetch(new URL(command.path, SnippetHostSetup.ORIGIN), {
      method: command.method,
      headers: { ...command.headers, 'content-type': SnippetHostSetup.JSON },
      body: JSON.stringify(command.body),
      redirect: 'error',
      signal: command.signal,
    })
    return { status: response.status, body: await response.text() }
  }
}
