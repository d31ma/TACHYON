@Service
export default class EventService {
  static _COLLECTION = 'hd-events';
  static _DEPLOYS = { kind: 'deploy' };
  static _CHANGE = { count: 7 };
  static _PAGE_SIZE = 2;
  static _TOPIC = 'app.render';
  static _GROUP = 'renderers';

  /** @type {DataSource} */
  @Needs
  dataSource;

  /** @returns {Promise<object>} */
  async report() {
    const collection = EventService._COLLECTION;
    await this.dataSource.createCollection(collection);
    const { id } = await this.dataSource.insert(collection, EventService._DEPLOYS);
    await this.dataSource.insert(collection, EventService._DEPLOYS);
    await this.dataSource.insert(collection, EventService._DEPLOYS);
    const found = /** @type {FyloDocument} */ (await this.dataSource.findById(collection, id));
    const patched = /** @type {FyloDocument} */ (await this.dataSource.patch(collection, id, EventService._CHANGE));
    const scrolled = await this.#countPages();
    const job = await this.#runJob();
    await this.dataSource.deleteById(collection, id);
    const gone = (await this.dataSource.findById(collection, id)) === null;
    const dropped = await this.dataSource.dropCollection(collection);
    return { found: found.kind, patched: patched.count, ...scrolled, job, gone, dropped };
  }

  /** @returns {Promise<{pages: number, seen: number}>} */
  async #countPages() {
    let pages = 0;
    let seen = 0;
    /** @type {string | null} */
    let cursor = null;
    let more = true;
    while (more) {
      const window = await this.dataSource.scroll(EventService._COLLECTION, EventService._DEPLOYS, cursor, EventService._PAGE_SIZE);
      pages += 1;
      seen += window.content.length;
      cursor = window.nextCursor;
      more = cursor !== null;
    }
    return { pages, seen };
  }

  /** @returns {Promise<unknown>} */
  async #runJob() {
    const topic = EventService._TOPIC;
    await this.dataSource.publish(topic, { app: 'web' }, 'render:web');
    const [delivery] = await this.dataSource.claim(topic, EventService._GROUP, 1, 30000);
    await this.dataSource.ack(topic, EventService._GROUP, delivery);
    return delivery.payload;
  }
}
