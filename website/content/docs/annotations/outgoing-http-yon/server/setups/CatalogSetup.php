<?php

#[Setup]
class CatalogSetup {
    public static function configure(mixed $execution): array {
        $exchange = 'exchange';
        return ['http' => ['catalog' => [$exchange => [CatalogTransportHelper::class, $exchange]]]];
    }
}
