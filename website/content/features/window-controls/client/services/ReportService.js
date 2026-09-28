@Service
export default class ReportService {
  /** @type {boolean} */
  canMinimize = typeof globalThis.tachyonWindow?.minimize === 'function'

  minimize() {
    globalThis.tachyonWindow?.minimize()
  }

  resize() {
    globalThis.tachyonWindow?.resize({ width: 1280, height: 800 })
  }
}
