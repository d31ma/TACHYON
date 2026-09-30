@Service
export default class ReportService {
  canMinimize: boolean = typeof globalThis.tachyonWindow?.minimize === 'function'

  minimize(): void {
    globalThis.tachyonWindow?.minimize()
  }

  resize(): void {
    globalThis.tachyonWindow?.resize({ width: 1280, height: 800 })
  }
}
