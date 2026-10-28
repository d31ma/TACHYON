@Delegate
class LedgerAdapterDelegate:
    TRANSACTION = 'transaction'
    ACTION = 'action'
    commands: list[str] = []
    rows: list[dict[str, object]] = [{'id': 1, 'status': 'open'}, {'id': 2, 'status': 'settled'}]
    sequence = 0

    @staticmethod
    async def execute(command: dict[str, object], execution: object) -> object:
        if command['operation'] == LedgerAdapterDelegate.TRANSACTION:
            identifier = command.get(LedgerAdapterDelegate.TRANSACTION)
            entry = command[LedgerAdapterDelegate.ACTION]
            if identifier:
                entry = command[LedgerAdapterDelegate.ACTION] + ':' + identifier
            LedgerAdapterDelegate.commands.append(entry)
            if command[LedgerAdapterDelegate.ACTION] != 'begin':
                return {'ok': True}
            LedgerAdapterDelegate.sequence += 1
            return {LedgerAdapterDelegate.TRANSACTION: f't{LedgerAdapterDelegate.sequence}'}
        query = command['query']
        transaction = command.get(LedgerAdapterDelegate.TRANSACTION, 'none')
        LedgerAdapterDelegate.commands.append(f"{query['kind']}@{transaction}")
        status = query['where'][0]['value']
        matching_rows = [row for row in LedgerAdapterDelegate.rows if row['status'] == status]
        if query['kind'] == 'count':
            return len(matching_rows)
        return matching_rows
