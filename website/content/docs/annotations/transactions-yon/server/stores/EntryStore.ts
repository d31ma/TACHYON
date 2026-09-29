@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
export default class EntryStore {
  async findByStatus(status: string): Promise<EntryEntity[] | void> {}
  async countByStatus(status: string): Promise<number | void> {}
}
