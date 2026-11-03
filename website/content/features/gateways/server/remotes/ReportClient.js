@HTTPClient
export default class ReportClient {
  /**
   * @param {string} period
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @Gateway({"class":"server/services/ReportService.java"})
  async summarize(period) {}

  /**
   * @param {number} months
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  async forecast(months) {}
}
