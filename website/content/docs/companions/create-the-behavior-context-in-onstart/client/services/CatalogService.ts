@Service
export default class CatalogService {
  @Needs
  lifecycle!: AbortSignal
  behaviors: BehaviorContext | null = null
  offset: number | undefined = 0

  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) input: BehaviorPageInput): number | undefined { return input.offset }

  @OnStart
  async initialize(): Promise<void> {
    this.behaviors = await Behaviors.create(ApplicationSetup, {
      partition: 'public-catalog',
      signal: this.lifecycle,
    })
    this.offset = this.page({ page: 1 })
  }
}
