import { mkdirSync } from 'node:fs'
import type { Subprocess } from 'bun'
import { RunnerSettings } from './RunnerSettings.ts'
import { SnippetHost } from './SnippetHost.ts'

export class RunnerProgram {
  static readonly DEFAULT_WEBSITE = '/opt/tachyon/website'
  static readonly DEFAULT_WEBSITE_PORT = '8000'
  static readonly LOCALHOST = '127.0.0.1'
  static readonly LANGUAGE = 'C.UTF-8'
  static readonly SIGNALS: NodeJS.Signals[] = ['SIGTERM', 'SIGINT']

  static async run(environment: Record<string, string | undefined>): Promise<number> {
    const settings = RunnerSettings.fromEnvironment(environment)
    mkdirSync(settings.temporaryFolder, { recursive: true })
    const host = new SnippetHost(settings)
    console.log(`Snippet host ready at ${host.url()}`)
    const website = RunnerProgram.startWebsite(environment, settings)
    for (const signal of RunnerProgram.SIGNALS) process.on(signal, () => website.kill(signal))
    const code = await website.exited
    await host.stop()
    return code
  }

  private static startWebsite(environment: Record<string, string | undefined>, settings: RunnerSettings): Subprocess {
    const website = environment.SNIPPET_HOST_WEBSITE ?? RunnerProgram.DEFAULT_WEBSITE
    const port = environment.WEBSITE_PORT ?? RunnerProgram.DEFAULT_WEBSITE_PORT
    return Bun.spawn([...settings.tyCommand, 'start', website, '--host', RunnerProgram.LOCALHOST, '--port', port], {
      env: { ...settings.toolEnvironment, PATH: settings.childPath, LANG: RunnerProgram.LANGUAGE, HOME: settings.temporaryFolder, TMPDIR: settings.temporaryFolder, TAC_CACHE_DIR: settings.cacheFolder },
      stdin: 'ignore',
      stdout: 'inherit',
      stderr: 'inherit',
    })
  }
}
