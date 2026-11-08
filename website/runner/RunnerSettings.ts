import { tmpdir } from 'node:os'
import path from 'node:path'

export class RunnerSettings {
  static readonly DEFAULT_PORT = 8100
  static readonly DEFAULT_PROJECTS = '/opt/tachyon/snippets'
  static readonly DEFAULT_TY = 'ty'
  static readonly DEFAULT_PATH = '/usr/local/bin:/usr/bin:/bin'
  static readonly SECONDS = 1000
  static readonly MINUTE = 60 * RunnerSettings.SECONDS
  static readonly TOOL_NAMES = ['DOTNET_ROOT', 'JAVA_HOME', 'RUSTUP_HOME', 'CARGO_HOME', 'GOROOT']

  port = RunnerSettings.DEFAULT_PORT
  projectsFolder = RunnerSettings.DEFAULT_PROJECTS
  temporaryFolder = path.join(tmpdir(), 'snippet-host')
  tyCommand: string[] = [RunnerSettings.DEFAULT_TY]
  childPath = RunnerSettings.DEFAULT_PATH
  cacheFolder = path.join(tmpdir(), 'tac-cache')
  toolEnvironment: Record<string, string> = {}
  runsPerMinute = 10
  runsPerDay = 200
  maxServers = 4
  maxActiveRuns = 4
  maxWaitingRuns = 16
  idleMilliseconds = RunnerSettings.MINUTE
  startMilliseconds = 30 * RunnerSettings.SECONDS
  runMilliseconds = 10 * RunnerSettings.SECONDS
  maxBodyBytes = 16 * 1024
  maxAnswerBytes = 64 * 1024

  static fromEnvironment(environment: Record<string, string | undefined>): RunnerSettings {
    const settings = new RunnerSettings()
    settings.port = Number(environment.SNIPPET_HOST_PORT ?? RunnerSettings.DEFAULT_PORT)
    settings.projectsFolder = environment.SNIPPET_HOST_PROJECTS ?? RunnerSettings.DEFAULT_PROJECTS
    settings.tyCommand = [environment.SNIPPET_HOST_TY ?? RunnerSettings.DEFAULT_TY]
    settings.childPath = environment.PATH ?? RunnerSettings.DEFAULT_PATH
    settings.cacheFolder = environment.TAC_CACHE_DIR ?? settings.cacheFolder
    settings.toolEnvironment = RunnerSettings.toolValues(environment)
    return settings
  }

  private static toolValues(environment: Record<string, string | undefined>): Record<string, string> {
    const present = RunnerSettings.TOOL_NAMES.filter(name => environment[name] !== undefined)
    return Object.fromEntries(present.map(name => [name, String(environment[name])]))
  }
}
