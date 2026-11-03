@Store(BookEntity, {"fields":["code","status"],"defaultSize":2,"maxSize":3,"setup":"ShelfSetup"})
export default class BookStore extends Repository<BookEntity, string> {
  async findByStatus(status: string): Promise<BookEntity[] | void> {}
  async countByStatus(status: string): Promise<number | void> {}
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"code","direction":"desc"}]})
  async excluding(status: string, @PageDefaults page: BehaviorPageInput): Promise<BookEntity[] | void> {}
}
