/** @import { ChartView } from '/models/ChartView.ts' */
@Service
export default class FeedService {
  static _SCALE = 10

  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  /** @type {number} */
  _turn = 0

  /** @returns {void} */
  next() {
    this._turn = this._turn + 1
    /** @type {ChartView} */
    const view = { title: `View ${this._turn}`, series: { points: [this._turn, this._turn * FeedService._SCALE] } }
    this.applicationEventPublisher.publishEvent(view, true)
  }

  /** @returns {void} */
  repeat() {
    /** @type {ChartView} */
    const view = { title: `View ${this._turn} again`, series: { points: [this._turn, this._turn * FeedService._SCALE] } }
    this.applicationEventPublisher.publishEvent(view, true)
  }
}
