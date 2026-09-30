import type { Report } from '/models/Report.ts'

@RequestRoute("/report")
@Public
class ReportController {
  @Needs
  readonly reportClient!: ReportClient;

  @GetRoute
  async getReport(): Promise<Report> {
    const summary = await this.reportClient.summarize('2026-Q3');
    const forecast = await this.reportClient.forecast(3);
    return { summary, forecast };
  }
}
