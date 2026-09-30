<?php
#[Description("Behavior conformance HandlerChecks")]
class HandlerChecks {
    public static function behaviorError(callable $action, string $message): BehaviorError {
        try {
            $action();
        } catch (BehaviorError $error) {
            return $error;
        }
        throw new Exception($message);
    }

    public static function checkConfiguration(array $values): void {
        $settings = Behaviors::properties(Settings::class, $values);
        if ($settings['size'] !== 25 || $settings['enabled'] !== true) throw new Exception('typed configuration');
        $namespaced = Behaviors::properties(Settings::class, ['yon' => ['catalog' => ['size' => 25, 'enabled' => true]]]);
        if ($namespaced !== $settings) throw new Exception('namespaced binding');
        $source = ['yon' => ['server' => ['host' => '127.0.0.1', 'port' => 9000], 'catalog' => ['size' => 25]]];
        $application = Behaviors::values(ApplicationValues::class, $source);
        if ($application['port'] !== 9000) throw new Exception('application values');
        if ($application['size'] !== 25) throw new Exception('application values');
        if ($application['redis'] !== 'redis://127.0.0.1:6379') throw new Exception('application values');
    }

    public static function checkValueLimits(): void {
        foreach ([str_repeat('x', 4096), str_repeat('é', 2048)] as $text) {
            $bound = Behaviors::values(ApplicationValues::class, ['yon' => ['redis' => ['url' => $text]]]);
            if ($bound['redis'] !== $text) throw new Exception('bounded selected value');
        }
        foreach ([str_repeat('x', 4097), str_repeat('é', 2049)] as $text) {
            $error = self::behaviorError(
                fn() => Behaviors::values(ApplicationValues::class, ['yon' => ['redis' => ['url' => $text]]]),
                'oversized selected value must fail',
            );
            if ($error->behaviorCode !== 'configuration-properties') throw $error;
            if (str_contains($error->getMessage(), $text)) throw $error;
        }
        $error = self::behaviorError(
            fn() => Behaviors::values(ApplicationValues::class, ['yon' => ['datasource' => ['password' => str_repeat('x', 4097)]]]),
            'oversized scalar accepted',
        );
        if ($error->behaviorCode !== 'configuration-properties') throw $error;
        $error = self::behaviorError(
            fn() => Behaviors::values(ApplicationValues::class, ['server' => ['port' => '9000']]),
            'a string port must fail',
        );
        if ($error->behaviorCode !== 'configuration-properties') throw $error;
    }

    public static function checkValidation(): void {
        $accepted = (new ValidationService())->accept(['name' => 'éé', 'price' => 20]);
        if ($accepted !== ['name' => 'éé', 'price' => 20]) throw new Exception('validation');
        $error = self::behaviorError(
            fn() => Behaviors::validate(ProductInput::class, ['name' => "\u{0085}\u{00a0}"]),
            'must fail',
        );
        if ($error->behaviorCode !== 'validation' || $error->path !== 'name.notBlank') throw $error;
    }

    public static function checkInactiveConfiguration($signal): void {
        $error = self::behaviorError(
            fn() => Behaviors::create(AdaptersSetup::class, ['partition' => 'catalog', 'signal' => $signal]),
            'must be inactive',
        );
        if ($error->behaviorCode !== 'inactive-configuration') throw $error;
    }

    public static function exerciseCache($catalog, $context, array $page): array {
        $first = $catalog->read($page);
        $cached = $catalog->read($page);
        $context->evict($catalog, 'read', [$page]);
        $fresh = $catalog->read($page);
        $catalog->replace(99, $page);
        if ($catalog->read($page)['calls'] !== 99) throw new Exception('cache put');
        try {
            $catalog->replace('fail', $page);
            throw new LogicException('must fail');
        } catch (Exception $error) {
            if ($error->getMessage() !== 'failed mutation') throw $error;
        }
        if ($catalog->read($page)['calls'] !== 99) throw new Exception('failed put changed cache');
        $catalog->remove($page);
        if ($catalog->read($page)['calls'] !== 3) throw new Exception('cache eviction');
        return [$first, $cached, $fresh];
    }

    public static function checkResilience($context): void {
        $resilient = Behaviors::construct(Resilient::class, ['behaviors' => $context]);
        if ($resilient->flaky() !== 3 || Resilient::$attempts !== 3) throw new Exception('retry contract');
        if ($resilient->bounded('ok') !== 'ok') throw new Exception('limit contract');
    }

    public static function checkQueries($context): void {
        $products = new ProductsStore();
        $products->behaviors = $context;
        if ($products->find_by_status('active') !== [['id' => 'one', 'status' => 'active']]) throw new Exception('query results');
        if ($products->countByStatus('active') !== 1) throw new Exception('query results');
        if ($products->available('active') !== true) throw new Exception('query results');
        if ($products->findById('one') !== ['id' => 'one', 'status' => 'active']) throw new Exception('repository findById');
        if ($products->findAll([]) !== [['id' => 'one', 'status' => 'active']]) throw new Exception('repository findAll');
        if ($products->save(['id' => 'one'])['operation'] !== 'save') throw new Exception('repository save');
        if ($products->deleteById('one') !== 1) throw new Exception('repository deleteById');
        if ($products->deleteByStatus('gone') !== 1) throw new Exception('derived delete');
    }

    public static function checkMapping($context): void {
        $summaries = Behaviors::construct(ProductSummaries::class, ['behaviors' => $context]);
        $summary = $summaries->toSummary(['detail' => ['id' => 7], 'name' => 'Anvil', 'extra' => 'dropped']);
        $expectedInKeyOrder = ['productId' => 7, 'title' => 'Anvil', 'absent' => null, 'kind' => 'product'];
        ksort($expectedInKeyOrder);
        if ($summary !== $expectedInKeyOrder) throw new Exception('mapping fields '.json_encode($summary));
        $refused = false;
        try {
            $summaries->toSummary(['not an object']);
        } catch (BehaviorError $error) {
            $refused = $error->behaviorCode === 'invalid';
        }
        if (!$refused) throw new Exception('a mapping reads one JSON object argument');
    }

    public static function isCancelled($catalog): bool {
        try {
            $catalog->remote();
        } catch (BehaviorError $error) {
            return $error->behaviorCode === 'cancelled';
        }
        return false;
    }

    public static function exercise($context, $signal): array {
        $catalog = new CatalogService();
        $catalog->behaviors = $context;
        $client = new CatalogClient();
        $client->behaviors = $context;
        $page = ['page' => 2, 'size' => 5];
        [$first, $cached, $fresh] = self::exerciseCache($catalog, $context, $page);
        $remote = $client->remote();
        $repository = $context->repository(ProductEntity::class);
        if ($client->remote() !== $remote || Backend::$exchanges !== 1) throw new Exception('cached exchange');
        if ($client->find(7) !== ['path' => '/products/7']) throw new Exception('path variable');
        $saved = $repository->save(['id' => 'one']);
        $found = $repository->find();
        $deleted = $repository->delete('one');
        self::checkResilience($context);
        self::checkQueries($context);
        self::checkMapping($context);
        $signal->abort();
        $cancelled = self::isCancelled($client);
        return [
            'first' => $first,
            'cached' => $cached,
            'fresh' => $fresh,
            'remote' => $remote,
            'saved' => $saved,
            'found' => $found,
            'deleted' => $deleted,
            'cancelled' => $cancelled,
            'authorizations' => Backend::$authorizations,
        ];
    }
}

