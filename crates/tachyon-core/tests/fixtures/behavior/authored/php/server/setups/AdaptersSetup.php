<?php
#[Setup]
#[WhenProfile("test","development")]
#[WhenSetting({"name":"YON_CATALOG_ENABLED","havingValue":"true","matchIfMissing":false})]
class AdaptersSetup {
    public static function configure($execution) {
        $cache = ['get' => [Backend::class, 'get'], 'set' => [Backend::class, 'set'], 'delete' => [Backend::class, 'delete']];
        $adapters = [
            'caches' => ['catalog' => $cache],
            'http' => ['api' => ['exchange' => [Backend::class, 'exchange']]],
            'entities' => ['data' => ['execute' => [Backend::class, 'execute']]],
            'authorize' => [Backend::class, 'authorize'],
            'authorizeEntity' => [Backend::class, 'authorizeEntity'],
        ];
        return $adapters;
    }
}

