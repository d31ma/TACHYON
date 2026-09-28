import type { CatalogCount } from '/models/CatalogCount.ts'

@Service
export default class CatalogService {
  applicationEventPublisher!: ApplicationEventPublisher
  count: number = 0

  @OnStart
  initialize(): void { this.announce() }

  private announce(): void {
    const counted: CatalogCount = { count: 5 }
    this.applicationEventPublisher.publishEvent(counted)
  }

  @OnEvent
  changed(value: CatalogCount): void { this.count = value.count }

  @OnStop
  cleanup(): void { this.count = 0 }
}