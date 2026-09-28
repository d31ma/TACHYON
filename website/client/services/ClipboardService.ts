import type { TaskScheduler } from '/models/TaskScheduler.ts'

@Service
@Lifetime({"value":"prototype"})
export default class ClipboardService {
  @Needs
  taskScheduler!: TaskScheduler
  @Needs
  lifecycle!: AbortSignal
  timer?: ReturnType<typeof setTimeout>
  copied = ''
  private copiedLabel = 'Copied'

  label(key: string): string {
    return this.labelFor(this.copied === key)
  }

  async copy(_event: unknown, key: string, text: string): Promise<void> {
    await this.writeWhileMounted(key, text)
    await this.renderWhileMounted()
    this.forgetLater()
  }

  @OnStop
  dispose(): void {
    clearTimeout(this.timer)
  }

  private labelFor(copied: boolean): string {
    if (copied) return this.copiedLabel
    return 'Copy'
  }

  private async writeWhileMounted(key: string, text: string): Promise<void> {
    if (this.lifecycle.aborted) return
    this.copied = key
    this.copiedLabel = await this.labelAfterWriting(text)
  }

  private labelAfterWriting(text: string): Promise<string> {
    return navigator.clipboard.writeText(text).then(() => 'Copied', () => 'Press ⌘C')
  }

  private async renderWhileMounted(): Promise<void> {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }

  private forgetLater(): void {
    clearTimeout(this.timer)
    this.timer = setTimeout(() => this.forgetCopied(), 2000)
  }

  private forgetCopied(): void {
    if (this.lifecycle.aborted) return
    this.copied = ''
    void this.taskScheduler.render().catch(() => {})
  }
}
