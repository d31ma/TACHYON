<?php

#[Setup]
class LedgerSetup {
    public static function configure(mixed $execution): array {
        $execute = 'execute';
        return [
            'entities' => ['ledger' => [$execute => [LedgerAdapterDelegate::class, $execute]]],
            'authorizeEntity' => [LedgerAdapterDelegate::class, 'authorize'],
        ];
    }
}
