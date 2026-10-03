@Service
export default class ShowcaseService {
  private static TAC_SUMMARY = 'One component runs on every target. The view and the stylesheet are the same everywhere. Pick a platform to see every language that can write the service there.'
  private static YON_SUMMARY = 'In every Yon language, a controller uses the same route annotations. Services, stores and other application modules stay ordinary code. The language itself puts them together.'

  layer = 'tac'

  pickLayer(_event: unknown, value: string): void {
    this.layer = value
  }

  summary(): string {
    return this.summaryFor(this.layer)
  }

  private summaryFor(layer: string): string {
    if (layer === 'tac') return ShowcaseService.TAC_SUMMARY
    return ShowcaseService.YON_SUMMARY
  }
}
