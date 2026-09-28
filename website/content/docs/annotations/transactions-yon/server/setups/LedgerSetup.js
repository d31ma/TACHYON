@Setup
class LedgerSetup {
  /** @type {string[]} */
  static commands = [];

  /** @returns {object} */
  static configure() {
    const rows = [{ id: 1, status: 'open' }, { id: 2, status: 'settled' }];
    let sequence = 0;
    const execute = (command) => {
      if (command.operation === 'transaction') {
        let entry = command.action;
        if (command.transaction) entry = `${command.action}:${command.transaction}`;
        LedgerSetup.commands.push(entry);
        if (command.action !== 'begin') return { ok: true };
        sequence += 1;
        return { transaction: `t${sequence}` };
      }
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
