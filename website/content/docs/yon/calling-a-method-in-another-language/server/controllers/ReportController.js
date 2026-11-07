/** @import { Report } from '/models/Report.ts' */
@RequestRoute("/report")
@Public
class ReportController {
  /** @type {ReportClient} */
  @Needs
  reportClient;

  /** @returns {Promise<Report>} */
  @GetRoute
  async getReport() {
    const summary = await this.reportClient.summarize('2026-Q3');
    const forecast = await this.reportClient.forecast(2);
    return { summary, forecast };
  }
}
