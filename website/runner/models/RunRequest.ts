export interface RunRequest {
  method: string
  path: string
  query: Record<string, string>
  body?: string
}
