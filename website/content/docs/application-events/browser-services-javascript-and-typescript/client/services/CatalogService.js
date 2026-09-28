/** @import { CatalogCount } from '/models/CatalogCount.ts' */
@Service
export default class CatalogService {
  /** @type {HTMLElement} */
  @Needs
  view;
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {TacTaskScheduler} */
  @Needs
  taskScheduler;
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  count = 0;

  @OnStart
  initialize() {
    this.view.dataset.ready = 'true';
    this._announce();
  }

  _announce() {
    /** @type {CatalogCount} */
    const counted = { count: this.count };
    this.applicationEventPublisher.publishEvent(counted, true);
  }

  /** @param {CatalogCount} value */
  @OnEvent({"replay":true})
  changed(value) { this.count = value.count; }

  @OnStop
  dispose() {
  }
}
