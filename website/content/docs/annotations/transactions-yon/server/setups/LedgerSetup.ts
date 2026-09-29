@Setup
export default class LedgerSetup {
  static commands: string[] = [];

  static configure(): BehaviorConfiguration {
    const rows = [{ id: 1, status: 'open' }, { id: 2, status: 'settled' }];
    let sequence = 0;
    const execute = (command: BehaviorEntityCommand | BehaviorTransactionCommand): BehaviorJsonValue => {
      if (command.operation === 'transaction') {
        if (command.action === 'begin') LedgerSetup.commands.push('begin');
        else LedgerSetup.commands.push(`${command.action}:${command.transaction}`);
        if (command.action !== 'begin') return { ok: true };
        sequence += 1;
        return { transaction: `t${sequence}` };
      }
      if (command.operation !== 'query') return [];
      const query = command.query;
      const transaction = command.transaction ?? 'none';
      LedgerSetup.commands.push(`${query.kind}@${transaction}`);
      const matchingRows = rows.filter((row) => row.status === query.where[0].value);
      if (query.kind === 'count') return matchingRows.length;
      return matchingRows;
    };
    return {
      authorizeEntity: () => true,
      entities: { ledger: { execute } },
    };
  }
}
