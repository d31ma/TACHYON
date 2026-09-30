interface CatalogChanged {
  count: number;
}
interface CatalogLoaded {
  items: string[];
}
interface CatalogSeen {
  count: number;
}

class ApplicationEvents {
  @Needs
  view!: HTMLElement;
  @Needs
  lifecycle!: AbortSignal;
  @Needs
  resourceLoader!: TacResourceLoader;
  @Needs
  taskScheduler!: TacTaskScheduler;
  applicationEventPublisher!: ApplicationEventPublisher;

  @OnStart()
  initialize(): void {
    this.view.dataset.ready = 'true';
    const changed: CatalogChanged = {count: 1};
    this.applicationEventPublisher.publishEvent(changed);
    this.applicationEventPublisher.publishEvent({items: ['ready']} satisfies CatalogLoaded, true);
  }

  @OnStop()
  async dispose(): Promise<void> {}

  @Order(-10)
  @OnEvent({replay: true, condition: "value.count >= 1", id: 'catalog.audit'})
  receive(value: CatalogChanged): void { void value.count; }

  @OnEvent
  forward(value: CatalogChanged): CatalogSeen { return {count: value.count}; }

  @InBackground()
  @OnEvent()
  async later(value: CatalogLoaded): Promise<void> { void value.items.length; }
}
void ApplicationEvents;

class InvalidApplicationEvents {
  applicationEventPublisher!: ApplicationEventPublisher;
  // @ts-expect-error Lifecycle hooks take no arguments.
  @OnStart()
  initialize(value: string): void { void value; }
  // @ts-expect-error Listener hooks receive exactly one payload.
  @OnEvent
  receive(first: CatalogChanged, second: CatalogChanged): void { void first; void second; }
  // @ts-expect-error OnEvent takes no event names: the model type of the parameter is the event.
  @OnEvent({event: 'catalog.changed'})
  named(value: CatalogChanged): void { void value; }
  send(): void {
    // @ts-expect-error publishEvent takes a model value, not an event name.
    this.applicationEventPublisher.publishEvent('catalog.changed', {count: 1});
  }
}
void InvalidApplicationEvents;
