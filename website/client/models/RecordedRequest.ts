export type RecordedRequest = {
  method: string
  path: string
  headers: Record<string, string>
  body?: string
}
