<?php

#[Setup]
class CatalogSetup {
    public static function configure($execution) {
        $cache = [
            'get' => [MemoryCacheHelper::class, 'get'],
            'set' => [MemoryCacheHelper::class, 'set'],
            'delete' => [MemoryCacheHelper::class, 'delete'],
        ];
        $authorize = 'authorize';
        return ['caches' => ['catalog' => $cache], $authorize => [MemoryCacheHelper::class, $authorize]];
    }
}
