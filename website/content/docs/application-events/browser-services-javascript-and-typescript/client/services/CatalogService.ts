import type { CatalogCount } from '/models/CatalogCount.ts';

@Service
export default class CatalogService {
  @Needs
  view!: HTMLElement;
  @Needs
  lifecycle!: AbortSignal;
  @Needs
  taskScheduler!: TacTaskScheduler;
  applicationEventPublisher!: ApplicationEventPublisher;
  count = 0;

  @OnStart
  initialize(): void {
    this.view.dataset.ready = 'true';
    this.announce();
  }

  private announce(): void {
    const counted: CatalogCount = { count: this.count };
    this.applicationEventPublisher.publishEvent(counted, true);
  }

  @OnEvent({"replay":true})
  changed(value: CatalogCount): void { this.count = value.count; }

  @OnStop
  dispose(): void {
  }
}
