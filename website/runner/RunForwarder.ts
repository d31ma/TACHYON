import type { ExampleServer } from './ExampleServer.ts'
import type { RunRequest } from './models/RunRequest.ts'
import type { RunResponse } from './models/RunResponse.ts'
import type { RunResult } from './models/RunResult.ts'

export class RunForwarder {
  static readonly LOCAL_HOST = 'localhost'
  static readonly JSON = 'application/json'
  static readonly TIMEOUT = 'TimeoutError'

  constructor(private readonly runMilliseconds: number, private readonly maxAnswerBytes: number) {}

  async forward(server: ExampleServer, request: RunRequest): Promise<RunResult> {
    const started = performance.now()
    const answer = await this.exchange(server, request, AbortSignal.timeout(this.runMilliseconds)).catch(error => this.timeoutOrThrow(error))
    return this.result(answer, Math.round(performance.now() - started))
  }

  private async exchange(server: ExampleServer, request: RunRequest, signal: AbortSignal): Promise<RunResponse> {
    const response = await fetch(this.url(server, request), {
      method: request.method,
      headers: { host: RunForwarder.LOCAL_HOST, accept: RunForwarder.JSON, 'content-type': RunForwarder.JSON },
      body: request.body,
      redirect: 'manual',
      signal,
    })
    const bytes = await this.cappedBytes(this.bodyOf(response))
    return {
      status: response.status,
      contentType: this.contentTypeOf(response),
      body: new TextDecoder().decode(bytes.subarray(0, this.maxAnswerBytes)),
      truncated: bytes.length > this.maxAnswerBytes,
    }
  }

  private url(server: ExampleServer, request: RunRequest): string {
    const url = new URL(request.path, `http://127.0.0.1:${server.port}`)
    url.search = new URLSearchParams(request.query).toString()
    return url.toString()
  }

  private bodyOf(response: Response): ReadableStream<Uint8Array> {
    return response.body ?? new Response('').body as ReadableStream<Uint8Array>
  }

  private contentTypeOf(response: Response): string {
    return response.headers.get('content-type') ?? ''
  }

  private async cappedBytes(body: ReadableStream<Uint8Array>): Promise<Uint8Array> {
    const parts: Uint8Array[] = []
    let size = 0
    const reader = body.getReader()
    for (let next = await reader.read(); !next.done && size <= this.maxAnswerBytes; next = await reader.read()) {
      parts.push(next.value)
      size += next.value.length
    }
    await reader.cancel()
    return new Uint8Array(await new Blob(parts).arrayBuffer())
  }

  private timeoutOrThrow(error: unknown): undefined {
    if (error instanceof DOMException && error.name === RunForwarder.TIMEOUT) return undefined
    throw error
  }

  private result(answer: RunResponse | undefined, durationMs: number): RunResult {
    if (answer === undefined) return { outcome: 'timeout', durationMs }
    return { outcome: 'ok', durationMs, response: answer }
  }
}
