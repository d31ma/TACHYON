<?php
#[Setup]
#[Settings({"prefix":"YON_CATALOG_"})]
#[WhenProfile("development")]
#[WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
class CatalogSetup {
    #[Min(1)]
    #[Max(100)]
    public int $pageSize = 20;
    public bool $cacheEnabled = false;
    public static function configure(mixed $execution): array { return []; }
}
