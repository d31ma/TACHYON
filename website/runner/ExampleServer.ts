import { cpSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import path from 'node:path'
import type { Subprocess } from 'bun'
import type { RunnerSettings } from './RunnerSettings.ts'

export class ExampleServer {
  static readonly READY = /ready at http:\/\/127\.0\.0\.1:(\d+)\//
  static readonly LOCALHOST = '127.0.0.1'
  static readonly PROJECT = 'project'
  static readonly TEMPORARY = 'tmp'
  static readonly STOP_MILLISECONDS = 5000
  static readonly LANGUAGE = 'C.UTF-8'
  static readonly ISOLATION = 'process'

  lastUsed = 0
  activeRuns = 0
  runsStarted = 0

  private constructor(readonly port: number, private readonly child: Subprocess<'ignore', 'pipe', 'inherit'>, readonly folder: string) {}

  static async start(projectFolder: string, settings: RunnerSettings): Promise<ExampleServer> {
    const folder = ExampleServer.copyProject(projectFolder, settings.temporaryFolder)
    const child = Bun.spawn(ExampleServer.command(folder, settings), {
      env: ExampleServer.environment(folder, settings),
      stdin: 'ignore',
      stdout: 'pipe',
      stderr: 'inherit',
    })
    const port = await ExampleServer.readyPort(child, settings.startMilliseconds).catch(error => ExampleServer.abandon(child, folder, error))
    return new ExampleServer(port, child, folder)
  }

  async stop(): Promise<void> {
    this.child.kill()
    await ExampleServer.exitedWithin(this.child, ExampleServer.STOP_MILLISECONDS)
    rmSync(this.folder, { recursive: true, force: true })
  }

  private static copyProject(projectFolder: string, temporaryFolder: string): string {
    mkdirSync(temporaryFolder, { recursive: true })
    const folder = mkdtempSync(path.join(temporaryFolder, 'example-'))
    cpSync(projectFolder, path.join(folder, ExampleServer.PROJECT), { recursive: true })
    mkdirSync(path.join(folder, ExampleServer.TEMPORARY))
    return folder
  }

  private static command(folder: string, settings: RunnerSettings): string[] {
    return [...settings.tyCommand, 'start', path.join(folder, ExampleServer.PROJECT), '--host', ExampleServer.LOCALHOST, '--port', '0']
  }

  private static environment(folder: string, settings: RunnerSettings): Record<string, string> {
    return {
      ...settings.passedEnvironment,
      PATH: settings.childPath,
      LANG: ExampleServer.LANGUAGE,
      HOME: folder,
      TMPDIR: path.join(folder, ExampleServer.TEMPORARY),
      TAC_CACHE_DIR: settings.cacheFolder,
      YON_ISOLATION: ExampleServer.ISOLATION,
    }
  }

  private static async readyPort(child: Subprocess<'ignore', 'pipe', 'inherit'>, timeout: number): Promise<number> {
    const lines = ExampleServer.lines(child.stdout)
    const port = await Promise.race([ExampleServer.portFrom(lines), ExampleServer.failAfter(timeout)])
    void ExampleServer.drain(lines)
    return port
  }

  private static async *lines(stream: ReadableStream<Uint8Array>): AsyncGenerator<string> {
    const decoder = new TextDecoder()
    let buffered = ''
    for await (const chunk of stream) {
      buffered += decoder.decode(chunk, { stream: true })
      const parts = buffered.split('\n')
      buffered = parts.pop() ?? ''
      yield* parts
    }
  }

  private static async portFrom(lines: AsyncGenerator<string>): Promise<number> {
    for await (const line of lines) {
      const match = line.match(ExampleServer.READY)
      if (match) return Number(match[1])
    }
    throw new Error('The example server stopped before it was ready')
  }

  private static async failAfter(milliseconds: number): Promise<never> {
    await Bun.sleep(milliseconds)
    throw new Error(`The example server was not ready within ${milliseconds} ms`)
  }

  private static async drain(lines: AsyncGenerator<string>): Promise<void> {
    for await (const _line of lines) continue
  }

  private static async abandon(child: Subprocess<'ignore', 'pipe', 'inherit'>, folder: string, error: unknown): Promise<never> {
    child.kill()
    await ExampleServer.exitedWithin(child, ExampleServer.STOP_MILLISECONDS)
    rmSync(folder, { recursive: true, force: true })
    throw error
  }

  private static async exitedWithin(child: Subprocess<'ignore', 'pipe', 'inherit'>, milliseconds: number): Promise<void> {
    const exited = await Promise.race([child.exited.then(() => true), Bun.sleep(milliseconds).then(() => false)])
    ExampleServer.forceUnless(child, exited)
    await child.exited
  }

  private static forceUnless(child: Subprocess<'ignore', 'pipe', 'inherit'>, exited: boolean): void {
    if (exited) return
    child.kill('SIGKILL')
  }
}
