@Service
export default class CatalogService {
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext | null} */
  behaviors = null;
  offset = 0;

  /**
   * @param {BehaviorPage} input
   * @returns {number}
   */
  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) input) { return input.offset; }

  @OnStart
  async initialize() {
    this.behaviors = await Behaviors.create(ApplicationSetup, {
      partition: 'public-catalog',
      signal: this.lifecycle,
    });
    this.offset = this.page({ page: 1 });
  }
}