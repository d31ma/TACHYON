@Service
export default class EventService {
  private static readonly COLLECTION = 'hd-events';
  private static readonly DEPLOYS = { kind: 'deploy' };
  private static readonly CHANGE = { count: 7 };
  private static readonly PAGE_SIZE = 2;
  private static readonly TOPIC = 'app.render';
  private static readonly GROUP = 'renderers';

  @Needs
  dataSource!: DataSource;

  async report() {
    const collection = EventService.COLLECTION;
    await this.dataSource.createCollection(collection);
    const { id } = await this.dataSource.insert(collection, EventService.DEPLOYS);
    await this.dataSource.insert(collection, EventService.DEPLOYS);
    await this.dataSource.insert(collection, EventService.DEPLOYS);
    const found = (await this.dataSource.findById(collection, id))!;
    const patched = (await this.dataSource.patch(collection, id, EventService.CHANGE))!;
    const scrolled = await this.countPages();
    const job = await this.runJob();
    await this.dataSource.deleteById(collection, id);
    const gone = (await this.dataSource.findById(collection, id)) === null;
    const dropped = await this.dataSource.dropCollection(collection);
    return { found: found.kind, patched: patched.count, ...scrolled, job, gone, dropped };
  }

  private async countPages() {
    let pages = 0;
    let seen = 0;
    let cursor: string | null = null;
    let more = true;
    while (more) {
      const window = await this.dataSource.scroll(EventService.COLLECTION, EventService.DEPLOYS, cursor, EventService.PAGE_SIZE);
      pages += 1;
      seen += window.content.length;
      cursor = window.nextCursor;
      more = cursor !== null;
    }
    return { pages, seen };
  }

  private async runJob() {
    const topic = EventService.TOPIC;
    await this.dataSource.publish(topic, { app: 'web' }, 'render:web');
    const [delivery] = await this.dataSource.claim(topic, EventService.GROUP, 1, 30000);
    await this.dataSource.ack(topic, EventService.GROUP, delivery);
    return delivery.payload;
  }
}
