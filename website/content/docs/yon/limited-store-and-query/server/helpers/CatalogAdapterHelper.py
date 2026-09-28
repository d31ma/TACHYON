@Helper
class CatalogAdapterHelper:
    ROWS: list[dict[str, object]] = [{'id': 1, 'status': 'active'}, {'id': 2, 'status': 'retired'}]

    @staticmethod
    async def execute(command: dict[str, object], execution: object) -> object:
        query = command['query']
        condition = query['where'][0]
        keep_unequal_rows = condition['operator'] == 'ne'
        matching_rows = []
        for row in CatalogAdapterHelper.ROWS:
            is_equal = row[condition['field']] == condition['value']
            is_match = is_equal
            if keep_unequal_rows:
                is_match = not is_equal
            if is_match:
                matching_rows.append(row)
        if query['kind'] == 'count':
            return len(matching_rows)
        return matching_rows[:query['page']['size']]
