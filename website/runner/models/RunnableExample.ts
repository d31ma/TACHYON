import type { RunnableRoute } from './RunnableRoute.ts'

export interface RunnableExample {
  schema: number
  runId: string
  source: string
  language: string
  routes: RunnableRoute[]
}
