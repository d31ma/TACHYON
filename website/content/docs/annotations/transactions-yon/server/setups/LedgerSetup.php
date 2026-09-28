<?php

#[Setup]
class LedgerSetup {
    public static function configure(mixed $execution): array {
        $execute = 'execute';
        return [
            'entities' => ['ledger' => [$execute => [LedgerAdapterHelper::class, $execute]]],
            'authorizeEntity' => [LedgerAdapterHelper::class, 'authorize'],
        ];
    }
}
