<?php
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
#[Service]
class CatalogService {
    private const PAGE = 'page';
    private const CALLS = 'calls';
    public BehaviorContext $behaviors;
    #[Cached]
    public function read(#[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] array $page): array {
        Backend::$calls++;
        return [self::CALLS => Backend::$calls, self::PAGE => $page];
    }
    #[Cached]
    public function price(#[CacheKey] string $sku, string $note): int {
        Backend::$prices++;
        return Backend::$prices;
    }
    #[UpdateCache({"method":"read"})]
    public function replace($value, array $page): array {
        if ($value === 'fail') throw new Exception('failed mutation');
        return [self::CALLS => $value, self::PAGE => $page];
    }
    #[ClearCache({"method":"read"})]
    public function remove(array $page): bool {
        return true;
    }
}

