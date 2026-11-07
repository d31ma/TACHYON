import type { Guide } from '/models/Guide.ts'

export interface DocumentationCatalogue {
  order: string[]
  topics: Record<string, Guide>
  redirects?: Record<string, string>
}
