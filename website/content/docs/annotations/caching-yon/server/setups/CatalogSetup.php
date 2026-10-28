<?php

#[Setup]
class CatalogSetup {
    public static function configure(mixed $execution): array {
        $cache = [
            'get' => [MemoryCacheDelegate::class, 'get'],
            'set' => [MemoryCacheDelegate::class, 'set'],
            'delete' => [MemoryCacheDelegate::class, 'delete'],
        ];
        $authorize = 'authorize';
        return ['caches' => ['catalog' => $cache], $authorize => [MemoryCacheDelegate::class, $authorize]];
    }
}
