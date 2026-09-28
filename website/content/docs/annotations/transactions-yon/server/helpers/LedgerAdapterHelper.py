@Helper
class LedgerAdapterHelper:
    TRANSACTION = 'transaction'
    ACTION = 'action'
    commands: list[str] = []
    rows: list[dict[str, object]] = [{'id': 1, 'status': 'open'}, {'id': 2, 'status': 'settled'}]
    sequence = 0

    @staticmethod
    async def execute(command: dict[str, object], execution: object) -> object:
        if command['operation'] == LedgerAdapterHelper.TRANSACTION:
            identifier = command.get(LedgerAdapterHelper.TRANSACTION)
            entry = command[LedgerAdapterHelper.ACTION]
            if identifier:
                entry = command[LedgerAdapterHelper.ACTION] + ':' + identifier
            LedgerAdapterHelper.commands.append(entry)
            if command[LedgerAdapterHelper.ACTION] != 'begin':
                return {'ok': True}
            LedgerAdapterHelper.sequence += 1
            return {LedgerAdapterHelper.TRANSACTION: f't{LedgerAdapterHelper.sequence}'}
        query = command['query']
        transaction = command.get(LedgerAdapterHelper.TRANSACTION, 'none')
        LedgerAdapterHelper.commands.append(f"{query['kind']}@{transaction}")
        status = query['where'][0]['value']
        matching_rows = [row for row in LedgerAdapterHelper.rows if row['status'] == status]
        if query['kind'] == 'count':
            return len(matching_rows)
        return matching_rows
