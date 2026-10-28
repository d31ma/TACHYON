<?php

#[Delegate]
class LedgerAdapterDelegate {
    private const TRANSACTION = 'transaction';
    private const ACTION = 'action';
    private const KIND = 'kind';
    public static array $commands = [];
    public static int $sequence = 0;

    public static function execute(array $command, mixed $execution): array|int {
        $rows = [['id' => 1, 'status' => 'open'], ['id' => 2, 'status' => 'settled']];
        if ($command['operation'] === self::TRANSACTION) {
            $entry = $command[self::ACTION];
            if (isset($command[self::TRANSACTION])) $entry = $command[self::ACTION] . ':' . $command[self::TRANSACTION];
            self::$commands[] = $entry;
            if ($command[self::ACTION] !== 'begin') return ['ok' => true];
            self::$sequence++;
            return [self::TRANSACTION => 't' . self::$sequence];
        }
        $query = $command['query'];
        $transaction = $command[self::TRANSACTION] ?? 'none';
        self::$commands[] = $query[self::KIND] . '@' . $transaction;
        $status = $query['where'][0]['value'];
        $matchingRows = [];
        foreach ($rows as $row) {
            if ($row['status'] === $status) $matchingRows[] = $row;
        }
        if ($query[self::KIND] === 'count') return count($matchingRows);
        return $matchingRows;
    }

    public static function authorize(array $command, mixed $execution): bool {
        return true;
    }
}
