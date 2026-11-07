@Setup
export default class CatalogSetup {
  /** @returns {BehaviorConfiguration} */
  static configure() {
    /** @type {BehaviorJsonObject[]} */
    const rows = [{ id: 1, status: 'active' }, { id: 2, status: 'retired' }];
    /** @param {BehaviorEntityCommand} command */
    const findRows = (command) => {
      if (command.operation !== 'query') return [];
      const query = command.query;
      const condition = query.where[0];
      const keepUnequalRows = condition.operator === 'ne';
      const matchingRows = rows.filter((row) => {
        const isEqual = row[condition.field] === condition.value;
        if (keepUnequalRows) return !isEqual;
        return isEqual;
      });
      if (query.kind === 'count') return matchingRows.length;
      return matchingRows.slice(0, query.page?.size);
    };
    return {
      authorizeEntity: (command) => command.entity === 'ProductEntity',
      entities: { catalog: { execute: findRows } },
    };
  }
}
