@Service
@Lifetime({"value":"prototype"})
export default class ScriptTitleService {
  @Needs
  private documentTitle!: DocumentTitle
  writes = 0

  @OnStart
  initialize(): void {
    this.documentTitle.set('Script start — 🦊')
  }

  write(_event: Event, title: string): void {
    this.documentTitle.set(title)
    this.writes++
  }

  oversized(): boolean {
    return this.accepted('x'.repeat(1025))
  }

  attempt(_event: Event, title: string): boolean {
    return this.accepted(title)
  }

  private accepted(title: string): boolean {
    try { this.documentTitle.set(title); return true }
    catch { return false }
  }
}
