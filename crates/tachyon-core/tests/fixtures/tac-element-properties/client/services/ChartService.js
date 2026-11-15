/** @import { ChartView } from '/models/ChartView.ts' */
@Service
export default class ChartService {
  /** @type {number[]} */
  points = [1, 2, 3]
  /** @type {ChartOptions | null} */
  options = { stacked: true, title: 'Load' }
  /** @type {string} */
  label = 'Load'
  /** @type {number} */
  count = 3
  /** @type {ChartView} */
  view = { title: 'View 0', series: { points: [] } }

  /** @returns {void} */
  addPoint() {
    this.points = [...this.points, this.points.length + 1]
    this.count = this.points.length
  }

  /** @returns {void} */
  clearOptions() {
    this.options = null
  }

  /** @param {ChartView} view */
  @OnEvent
  receive(view) {
    this.view = view
  }
}
