@Service
class ReportService {
  /**
   * @param {string} period
   * @returns {{ period: string, total: number, language: string }}
   */
  summarize(period) {
    return { period, total: 42, language: 'javascript' }
  }
}
