<?php

#[Delegate]
class CatalogAdapterDelegate {
    public static function execute(array $command, mixed $execution): array|int {
        $rows = [['id' => 1, 'status' => 'active'], ['id' => 2, 'status' => 'retired']];
        $query = $command['query'];
        $condition = $query['where'][0];
        $keepUnequalRows = $condition['operator'] === 'ne';
        $matchingRows = [];
        foreach ($rows as $row) {
            $isEqual = $row[$condition['field']] === $condition['value'];
            $isMatch = $keepUnequalRows ? !$isEqual : $isEqual;
            if ($isMatch) $matchingRows[] = $row;
        }
        if ($query['kind'] === 'count') return count($matchingRows);
        return array_slice($matchingRows, 0, $query['page']['size']);
    }

    public static function authorize(array $command, mixed $execution): bool {
        return $command['entity'] === 'ProductEntity';
    }
}
