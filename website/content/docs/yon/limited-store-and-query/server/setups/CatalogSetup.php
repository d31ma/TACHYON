<?php

#[Setup]
class CatalogSetup {
    public static function configure(mixed $execution): array {
        $execute = 'execute';
        return [
            'entities' => ['catalog' => [$execute => [CatalogAdapterHelper::class, $execute]]],
            'authorizeEntity' => [CatalogAdapterHelper::class, 'authorize'],
        ];
    }
}
