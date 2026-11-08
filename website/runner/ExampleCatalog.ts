import { existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import type { RunnableExample } from './models/RunnableExample.ts'
import type { RunnableRoute } from './models/RunnableRoute.ts'

export class ExampleCatalog {
  static readonly DESCRIPTION = 'runnable.json'
  static readonly PROJECT = 'project'
  static readonly RUN_ID = /^[0-9a-f]{16}$/
  static readonly SEGMENT = '[^/?#]+'
  static readonly VARIABLE = /\\\{[a-z0-9_]+\\\}/gi
  static readonly SPECIAL = /[.*+?^${}()|[\]\\]/g

  private readonly examples: Map<string, RunnableExample>

  constructor(private readonly projectsFolder: string) {
    this.examples = new Map(this.loadExamples().map(example => [example.runId, example]))
  }

  size(): number {
    return this.examples.size
  }

  find(runId: string): RunnableExample | undefined {
    return this.examples.get(this.knownId(runId))
  }

  allows(example: RunnableExample, method: string, requestPath: string): boolean {
    return example.routes.some(route => this.matches(route, method, requestPath))
  }

  projectFolder(example: RunnableExample): string {
    return path.join(this.projectsFolder, example.runId, ExampleCatalog.PROJECT)
  }

  private knownId(runId: string): string {
    if (!ExampleCatalog.RUN_ID.test(runId)) return ''
    return runId
  }

  private matches(route: RunnableRoute, method: string, requestPath: string): boolean {
    return route.method === method && this.pattern(route.path).test(requestPath)
  }

  private pattern(template: string): RegExp {
    const escaped = template.replace(ExampleCatalog.SPECIAL, '\\$&')
    return new RegExp(`^${escaped.replace(ExampleCatalog.VARIABLE, ExampleCatalog.SEGMENT)}$`)
  }

  private loadExamples(): RunnableExample[] {
    return this.runIds().map(runId => this.readExample(runId))
  }

  private runIds(): string[] {
    if (!existsSync(this.projectsFolder)) return []
    return readdirSync(this.projectsFolder).filter(name => ExampleCatalog.RUN_ID.test(name))
  }

  private readExample(runId: string): RunnableExample {
    return JSON.parse(readFileSync(path.join(this.projectsFolder, runId, ExampleCatalog.DESCRIPTION), 'utf8'))
  }
}
