<?php

#[Setup]
class CatalogSetup {
    public static function configure(mixed $execution): array {
        $execute = 'execute';
        return [
            'entities' => ['catalog' => [$execute => [CatalogAdapterDelegate::class, $execute]]],
            'authorizeEntity' => [CatalogAdapterDelegate::class, 'authorize'],
        ];
    }
}
