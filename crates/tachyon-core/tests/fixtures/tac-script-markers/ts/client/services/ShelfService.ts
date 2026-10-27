@Service
export default class ShelfService {
  private static readonly ACTIVE = 'active';
  private static readonly FULL = 'the shelf is full';
  @Needs
  bookStore!: BookStore;
  @Needs
  lifecycle!: AbortSignal;
  behaviors!: BehaviorContext;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(ShelfSetup, { partition: 'shelf', signal: this.lifecycle });
  }

  async report(): Promise<BehaviorJsonValue> {
    const active = await this.bookStore.findByStatus(ShelfService.ACTIVE);
    const count = await this.bookStore.countByStatus(ShelfService.ACTIVE);
    const others = await this.bookStore.excluding(ShelfService.ACTIVE, {});
    const full = await this.bookStore.excluding(ShelfService.ACTIVE, { size: 3 });
    return { active, count, others, full };
  }

  async oversized(): Promise<BookEntity[] | void> {
    return this.bookStore.excluding(ShelfService.ACTIVE, { size: 4 });
  }

  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["code"]}) input: BehaviorPageInput): BehaviorPageInput {
    return input;
  }

  @AllOrNothing({"adapter":"shelf"})
  async shelve(code: string, status: string): Promise<BehaviorJsonValue> {
    return this.behaviors.repository(BookEntity).save({ code, status });
  }

  @AllOrNothing({"adapter":"shelf"})
  async refuse(code: string): Promise<BehaviorJsonValue> {
    await this.behaviors.repository(BookEntity).save({ code, status: 'draft' });
    throw Error(ShelfService.FULL);
  }
}
