declare const tachyonHost:
  | { invoke(capability: string, payload?: unknown): Promise<unknown> }
  | undefined

export default class {
  development: boolean = document.body.dataset.tachyonEnvironment === 'development'

  runtime: string = `TypeScript on ${navigator.platform || 'the web'}, ${
    navigator.hardwareConcurrency ?? '?'
  } cores`

  native: boolean = document.body.dataset.platform === 'native'

  async quit(): Promise<void> {
    await tachyonHost?.invoke('app.quit')
  }

  @onMount
  title() {
    document.title = 'Tachyon — one project, web and native'
  }
}
