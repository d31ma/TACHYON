@RequestRoute("/report")
@Public
class ReportController {
  /** @type {ReportClient} */
  @Needs
  reportClient;

  /** @returns {Promise<Report>} */
  @GetRoute
  async getReport() {
    const summary = await this.reportClient.summarize('2026-Q3')
    const forecast = await this.reportClient.forecast(3)
    return { summary, forecast }
  }
}
