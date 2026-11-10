import { HostRefusal } from './HostRefusal.ts'
import type { RunRequest } from './models/RunRequest.ts'

export class RunRequestReader {
  static readonly METHODS = new Set(['GET', 'POST', 'PUT', 'PATCH', 'DELETE'])
  static readonly PATH = /^\/[A-Za-z0-9._~!$&'()*+,;=:@%/-]{0,255}$/
  static readonly PARENT = /(^|\/)\.\.(\/|$)/
  static readonly MAX_QUERY_ENTRIES = 16
  static readonly MAX_QUERY_TEXT = 256
  static readonly BAD_REQUEST = 400
  static readonly TOO_LARGE = 413
  static readonly ENVELOPE_BYTES = 1024

  constructor(private readonly maxBodyBytes: number) {}

  async read(request: Request): Promise<RunRequest> {
    const text = await request.text()
    this.refuseLargerThan(text, this.maxBodyBytes + RunRequestReader.ENVELOPE_BYTES)
    const value = this.parse(text)
    return { method: this.method(value.method), path: this.path(value.path), query: this.query(value.query), body: this.body(value.body) }
  }

  private refuseLargerThan(text: string | undefined, limit: number): void {
    if (new TextEncoder().encode(text ?? '').length <= limit) return
    throw new HostRefusal(RunRequestReader.TOO_LARGE, { code: 'too-large', message: `The request is larger than ${limit} bytes.` })
  }

  private parse(text: string): Record<string, unknown> {
    try {
      return this.object(JSON.parse(text))
    } catch {
      throw this.invalid('The request must be a JSON object.')
    }
  }

  private object(value: unknown): Record<string, unknown> {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new Error('not an object')
    return value as Record<string, unknown>
  }

  private method(value: unknown): string {
    if (typeof value === 'string' && RunRequestReader.METHODS.has(value)) return value
    throw this.invalid('method must be GET, POST, PUT, PATCH or DELETE.')
  }

  private path(value: unknown): string {
    if (typeof value === 'string' && RunRequestReader.PATH.test(value) && !RunRequestReader.PARENT.test(value)) return value
    throw this.invalid('path must start with / and use at most 256 URL path characters.')
  }

  private query(value: unknown): Record<string, string> {
    const entries = Object.entries(this.object(value ?? {}))
    if (entries.length <= RunRequestReader.MAX_QUERY_ENTRIES && entries.every(([name, text]) => this.shortText(name) && this.shortText(text))) {
      return Object.fromEntries(entries) as Record<string, string>
    }
    throw this.invalid(`query must hold at most ${RunRequestReader.MAX_QUERY_ENTRIES} text values of at most ${RunRequestReader.MAX_QUERY_TEXT} characters.`)
  }

  private shortText(value: unknown): boolean {
    return typeof value === 'string' && value.length <= RunRequestReader.MAX_QUERY_TEXT
  }

  private body(value: unknown): string | undefined {
    if (value !== undefined && typeof value !== 'string') throw this.invalid('body must be text.')
    this.refuseLargerThan(value, this.maxBodyBytes)
    return value
  }

  private invalid(message: string): HostRefusal {
    return new HostRefusal(RunRequestReader.BAD_REQUEST, { code: 'invalid-request', message })
  }
}
