import type { RecordedRun } from '/models/RecordedRun.ts'

@HTTPClient("/shared/data/runs")
export default class RecordedRunClient {
  @GetRequest("/{file}")
  async run(@PathVariable("file") file: string): Promise<RecordedRun | void> {}
}
