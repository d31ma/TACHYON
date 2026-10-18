<?php
final class Service {
    public BehaviorContext $behaviors;
}

final class Orders {
    public BehaviorContext $behaviors;
}

final class Ledger {
    public BehaviorContext $behaviors;
}

final class RestHolder {
    public static array $seen = [];
}

final class NotesSetup {
    public static int $configured = 0;

    public static function configure(BehaviorExecution $execution): array {
        self::$configured++;
        $notes = new ArrayObject(['Ada']);
        $exchange = function(array $request) use ($notes): array {
            if (array_key_exists('body', $request)) $notes->append($request['body']);
            return ['items' => $notes->getArrayCopy()];
        };
        return ['http' => ['notes' => ['exchange' => $exchange]]];
    }
}

final class RuntimeContract {
    public static array $failures = [];
    public static array $state = ['hit' => ['hit' => false], 'authorization' => 0, 'calls' => 0, 'saved' => 0];
    public static int $attempts = 0;
    public static array $log = [];
    public static int $sequence = 0;

    public static function loadRuntime(): void {
        $source = file_get_contents(dirname(__DIR__, 3).'/src/behavior/polyglot/php_runtime.php');
        $placeholders = ['__BEHAVIOR_CONFIGURATIONS__', '__BEHAVIOR_CONSTRUCTORS__', '__BEHAVIOR_RECORDS__', '__BEHAVIOR_ENVIRONMENT__', '__BEHAVIOR_MODULE_ID__'];
        $values = ["case 'NotesSetup': return NotesSetup::configure(\$execution);", '', '{"NotesSetup":{"configuration":true,"methods":{}}}', 'yon', str_repeat('a', 64)];
        eval(str_replace($placeholders, $values, $source));
    }

    public static function rejectBehavior(string $label, callable $action, string $code = 'invalid'): void {
        try {
            $action();
            self::$failures[] = "$label: unexpectedly accepted";
        } catch (BehaviorError $error) {
            if ($error->behaviorCode !== $code) self::$failures[] = "$label: wrong code {$error->behaviorCode}";
        } catch (Throwable $error) {
            self::$failures[] = "$label: leaked ".get_class($error);
        }
    }

    public static function serviceContext(BehaviorSignal $signal, array $entity): BehaviorContext {
        $record = ['name' => Service::class, 'cache' => ['name' => 'data', 'ttlMs' => 1000, 'maxBytes' => 1024, 'maxEntries' => 10], 'methods' => [
            'read' => ['cacheable' => ['keyArgs' => [0]]],
            'remote' => ['httpExchange' => ['client' => 'api', 'method' => 'GET', 'path' => '/products']],
        ]];
        $context = new BehaviorContext([Service::class => $record, 'Product' => $entity], ['partition' => 'tenant', 'signal' => $signal]);
        $context->configure(fn($execution) => self::serviceAdapters());
        return $context;
    }

    public static function serviceAdapters(): array {
        return [
            'authorize' => function ($call, $execution) {
                self::$state['authorization']++;
                return true;
            },
            'caches' => ['data' => [
                'get' => fn($key, $execution) => self::$state['hit'],
                'set' => function ($key, $value, $execution) {
                    self::$state['hit'] = ['hit' => true, 'value' => $value, 'expiresAt' => $execution->expiresAt];
                },
                'delete' => function ($key, $execution) {
                    self::$state['hit'] = ['hit' => false];
                },
            ]],
            'http' => ['api' => ['exchange' => function ($request, $execution) {
                if ($request['redirect'] !== 'error' || $request['credentials'] !== 'omit') throw new Exception('transport contract');
                return 'transport';
            }]],
            'entities' => ['store' => ['execute' => function ($command, $execution) {
                self::$state['saved']++;
                return $command['operation'];
            }]],
        ];
    }

    public static function read(BehaviorContext $context, Service $service) {
        return $context->invoke($service, Service::class, 'read', [['id' => 1]], function ($args) {
            self::$state['calls']++;
            return $args[0];
        });
    }

    public static function checkCache(BehaviorContext $context, Service $service): void {
        if (self::read($context, $service) !== ['id' => 1]) throw new Exception('cache/auth contract');
        if (self::read($context, $service) !== ['id' => 1]) throw new Exception('cache/auth contract');
        if (self::$state['authorization'] !== 2 || self::$state['calls'] !== 1) throw new Exception('cache/auth contract');
        $context->evict($service, 'read', [['id' => 1]]);
        self::read($context, $service);
        if (self::$state['calls'] !== 2) throw new Exception('eviction contract');
        $expiresAt = (int)floor(microtime(true) * 1000) + 100000;
        self::$state['hit'] = ['hit' => true, 'value' => null, 'expiresAt' => $expiresAt];
        self::rejectBehavior('cache excessive TTL', fn() => self::read($context, $service));
    }

    public static function checkHttp(BehaviorContext $context, Service $service): void {
        $remote = fn($descriptor) => $context->invoke($service, Service::class, 'remote', [], fn($args) => $descriptor);
        self::rejectBehavior('GET body', fn() => $remote(['body' => []]));
        self::rejectBehavior('Cookie header', fn() => $remote(['headers' => ['Cookie' => 'x']]));
        if ($remote([]) !== 'transport') throw new Exception('HTTP contract');
    }

    public static function checkRepository(BehaviorContext $context, array $entity): void {
        $repository = $context->repository('Product');
        self::rejectBehavior('save missing id', fn() => $repository->save([]));
        self::rejectBehavior('save unsafe id', fn() => $repository->save(['id' => true]));
        self::rejectBehavior('unsupported entity operation', fn() => $context->entityOperation($entity, 'drop-table', []));
        self::rejectBehavior('direct entity save bypass', fn() => $context->entityOperation($entity, 'save', []));
        self::rejectBehavior('direct entity find bypass', fn() => $context->entityOperation($entity, 'find', ['size' => 10000]));
        if ($repository->save(['id' => 'one']) !== 'save') throw new Exception('entity contract');
        if ($repository->delete('one') !== 'delete') throw new Exception('entity contract');
    }

    public static function checkBoundaries(): void {
        self::rejectBehavior('whitespace partition', fn() => new BehaviorContext([], ['partition' => '  ', 'signal' => new BehaviorSignal()]));
        $pageRules = ['defaultSize' => 20, 'maxSize' => 100, 'sort' => []];
        self::rejectBehavior('null page', fn() => BehaviorBoundary::page(['page' => null], $pageRules));
        self::rejectBehavior('null offset', fn() => BehaviorBoundary::page(['offset' => null], $pageRules));
        self::rejectBehavior('nonfinite JSON', fn() => BehaviorBoundary::copy(INF));
        self::rejectBehavior('invalid UTF8 JSON', fn() => BehaviorBoundary::copy("\xff"));
        self::rejectBehavior('large JSON', fn() => BehaviorBoundary::copy(str_repeat('x', 1048577)));
        $optionalSchema = ['fields' => ['name' => ['size' => ['min' => 1, 'max' => 16]]]];
        self::rejectBehavior('empty array is not a DTO object', fn() => BehaviorValidation::apply($optionalSchema, []), 'validation');
        $validated = BehaviorValidation::apply($optionalSchema, new stdClass());
        if (!($validated instanceof stdClass)) throw new Exception('empty DTO object shape lost');
    }

    public static function checkRejections(): void {
        $schema = ['fields' => ['name' => ['notBlank' => true, 'size' => ['min' => 1, 'max' => 80]], 'level' => ['notNull' => true, 'min' => 1, 'max' => 9]]];
        $input = ['name' => '', 'level' => '3'];
        try {
            BehaviorValidation::apply($schema, $input);
            self::$failures[] = 'apply accepted broken input';
        } catch (BehaviorRejection $error) {
            self::$failures[] = 'apply raised a rejection';
        } catch (BehaviorError $error) {
            if ($error->path !== 'name.notBlank') self::$failures[] = "apply path {$error->path}";
        }
        $expected = [['field' => 'name', 'rule' => 'NotEmpty'], ['field' => 'name', 'rule' => 'Length'], ['field' => 'level', 'rule' => 'Min'], ['field' => 'level', 'rule' => 'Max']];
        self::expectRejection('every broken rule', fn() => BehaviorValidation::check($schema, $input), $expected, 'name.notBlank');
        self::expectRejection('a list is not an object', fn() => BehaviorValidation::check($schema, [1]), [['field' => '', 'rule' => 'Checked']], '$.object');
        self::expectRejection('an empty body is an empty object', fn() => BehaviorValidation::check($schema, []), [['field' => 'name', 'rule' => 'NotEmpty'], ['field' => 'level', 'rule' => 'Required']], 'name.notBlank');
        self::rejectBehavior('an unknown schema', fn() => BehaviorValidation::check(null, $input), 'validation');
        if (BehaviorValidation::check($schema, ['name' => 'a', 'level' => 3]) !== ['level' => 3, 'name' => 'a']) self::$failures[] = 'check changed valid input';
    }

    public static function expectRejection(string $label, callable $action, array $violations, string $path): void {
        try {
            $action();
            self::$failures[] = "$label: unexpectedly accepted";
        } catch (BehaviorRejection $error) {
            if ($error->violations !== $violations) self::$failures[] = "$label: violations ".json_encode($error->violations);
            if ($error->behaviorCode !== 'validation' || $error->path !== $path) self::$failures[] = "$label: {$error->behaviorCode} {$error->path}";
        } catch (Throwable $error) {
            self::$failures[] = "$label: leaked ".get_class($error);
        }
    }

    public static function resilientContext(): BehaviorContext {
        $resilient = ['name' => 'Orders', 'methods' => [
            'flaky' => ['retryable' => ['maxRetries' => 3, 'delayMs' => 0, 'multiplier' => 1, 'maxDelayMs' => 100, 'jitterMs' => 0]],
            'guarded' => ['concurrencyLimit' => ['limit' => 1, 'policy' => 'reject']],
        ]];
        $context = new BehaviorContext([Orders::class => $resilient], ['partition' => 'tenant', 'signal' => new BehaviorSignal()]);
        $context->configure(fn($execution) => []);
        return $context;
    }

    public static function checkRetries(BehaviorContext $context, Orders $orders): void {
        self::$attempts = 0;
        $flaky = $context->invoke($orders, Orders::class, 'flaky', [], function ($args) {
            self::$attempts++;
            if (self::$attempts < 3) throw new Exception('transient');
            return 'ok';
        });
        $attempts = self::$attempts;
        if ($flaky !== 'ok' || $attempts !== 3) throw new Exception("retry contract: $attempts attempts");
        self::$attempts = 0;
        try {
            $context->invoke($orders, Orders::class, 'flaky', [], function ($args) {
                self::$attempts++;
                throw new Exception('permanent');
            });
            self::$failures[] = 'exhausted retries: unexpectedly accepted';
        } catch (Exception $error) {
        }
        $attempts = self::$attempts;
        if ($attempts !== 4) throw new Exception("retry bound: $attempts attempts");
    }

    public static function checkConcurrencyLimit(BehaviorContext $context, Orders $orders): void {
        $nested = fn($args) => $context->invoke($orders, Orders::class, 'guarded', [], fn($inner) => 'nested');
        self::rejectBehavior('re-entrant concurrency limit', fn() => $context->invoke($orders, Orders::class, 'guarded', [], $nested), 'capacity');
        $free = $context->invoke($orders, Orders::class, 'guarded', [], fn($args) => 'free');
        if ($free !== 'free') throw new Exception('limit contract: slot not released');
    }

    public static function checkResilience(): void {
        $context = self::resilientContext();
        $orders = new Orders();
        $orders->behaviors = $context;
        self::checkRetries($context, $orders);
        self::checkConcurrencyLimit($context, $orders);
    }

    public static function executeLedger(array $command, $execution): array {
        if ($command['operation'] !== 'transaction') {
            self::$log[] = $command['operation'].'@'.($command['transaction'] ?? 'none');
            return ['id' => 'one'];
        }
        $entry = $command['action'];
        if (isset($command['transaction'])) $entry .= ':'.$command['transaction'];
        self::$log[] = $entry;
        if ($command['action'] !== 'begin') return ['ok' => true];
        self::$sequence++;
        $sequence = self::$sequence;
        return ['transaction' => "t$sequence"];
    }

    public static function ledgerContext(): BehaviorContext {
        $transactional = ['name' => 'Ledger', 'methods' => ['post' => ['transactional' => ['adapter' => 'ledger']]]];
        $ledgerEntity = ['name' => 'Entry', 'entity' => ['adapter' => 'ledger', 'id' => 'id'], 'table' => 'entries', 'methods' => []];
        $records = [Ledger::class => $transactional, 'Entry' => $ledgerEntity];
        $context = new BehaviorContext($records, ['partition' => 'tenant', 'signal' => new BehaviorSignal()]);
        $execute = fn($command, $execution) => self::executeLedger($command, $execution);
        $context->configure(fn($execution) => ['entities' => ['ledger' => ['execute' => $execute]]]);
        return $context;
    }

    public static function post(BehaviorContext $context, Ledger $ledger, bool $fail) {
        return $context->invoke($ledger, Ledger::class, 'post', [], function ($args) use ($context, $fail) {
            $context->repository('Entry')->save(['id' => 'one']);
            if ($fail) throw new Exception('domain rejected the entry');
            return 'posted';
        });
    }

    public static function checkTransactions(): void {
        self::$log = [];
        self::$sequence = 0;
        $context = self::ledgerContext();
        $ledger = new Ledger();
        $ledger->behaviors = $context;
        if (self::post($context, $ledger, false) !== 'posted') throw new Exception('transaction contract: '.implode(',', self::$log));
        if (self::$log !== ['begin', 'save@t1', 'commit:t1']) throw new Exception('transaction contract: '.implode(',', self::$log));
        self::$log = [];
        try {
            self::post($context, $ledger, true);
            self::$failures[] = 'a failing transactional body was accepted';
        } catch (Exception $error) {
        }
        if (self::$log !== ['begin', 'save@t2', 'rollback:t2']) throw new Exception('rollback contract: '.implode(',', self::$log));
        self::$log = [];
        $context->repository('Entry')->save(['id' => 'two']);
        if (self::$log !== ['save@none']) throw new Exception('transaction leaked past its method: '.implode(',', self::$log));
    }

    public static function executeJoined(array $command, $execution): array {
        if ($command['operation'] === 'transaction') return self::executeLedger($command, $execution);
        self::$log[] = $command['operation'].'@'.($command['transaction'] ?? 'none');
        if ($command['value']['id'] === 'broken') throw new Exception('the second write failed');
        return ['id' => $command['value']['id']];
    }

    public static function joinedContext(array $records, string $partition): BehaviorContext {
        $context = new BehaviorContext($records, ['partition' => $partition, 'signal' => new BehaviorSignal()]);
        $adapter = ['execute' => fn($command, $execution) => self::executeJoined($command, $execution)];
        $context->configure(fn($execution) => ['entities' => ['ledger' => $adapter, 'audit' => $adapter]]);
        return $context;
    }

    public static function settle(BehaviorContext $service, BehaviorContext $store, string $second) {
        $ledger = new Ledger();
        $ledger->behaviors = $service;
        return $service->invoke($ledger, Ledger::class, 'post', [], function ($args) use ($store, $second) {
            $store->repository('Entry')->save(['id' => 'first']);
            $store->repository('Note')->save(['id' => 'note']);
            $store->repository('Entry')->save(['id' => $second]);
            return 'settled';
        });
    }

    public static function checkJoinedTransactions(): void {
        self::$log = [];
        self::$sequence = 0;
        $service = self::joinedContext([Ledger::class => ['name' => 'Ledger', 'methods' => ['post' => ['transactional' => ['adapter' => 'ledger']]]]], 'ledger');
        $entry = ['name' => 'Entry', 'entity' => ['adapter' => 'ledger', 'id' => 'id'], 'table' => 'entries', 'methods' => []];
        $note = ['name' => 'Note', 'entity' => ['adapter' => 'audit', 'id' => 'id'], 'table' => 'notes', 'methods' => []];
        $store = self::joinedContext(['Entry' => $entry, 'Note' => $note], 'entries');
        if (self::settle($service, $store, 'second') !== 'settled') throw new Exception('joined transaction: '.implode(',', self::$log));
        if (self::$log !== ['begin', 'save@t1', 'save@none', 'save@t1', 'commit:t1']) throw new Exception('joined transaction: '.implode(',', self::$log));
        self::$log = [];
        try {
            self::settle($service, $store, 'broken');
            self::$failures[] = 'a failed second write was accepted';
        } catch (Exception $error) {
        }
        if (self::$log !== ['begin', 'save@t2', 'save@none', 'save@t2', 'rollback:t2']) throw new Exception('joined rollback: '.implode(',', self::$log));
    }

    public static function restContext(): BehaviorContext {
        $context = new BehaviorContext([], ['partition' => 'tenant', 'signal' => new BehaviorSignal()]);
        $exchange = function ($request, $execution) {
            if ($request['redirect'] !== 'error' || $request['credentials'] !== 'omit') throw new Exception('transport contract');
            RestHolder::$seen[] = $request['method'].' '.$request['path'];
            return ['ok' => true, 'body' => $request['body'] ?? null];
        };
        $context->configure(fn($execution) => ['http' => ['catalog' => ['exchange' => $exchange]]]);
        return $context;
    }

    public static function checkRestClient(): void {
        $context = self::restContext();
        $rest = $context->restClient('catalog');
        $read = $rest->get('/products')->query(['page' => 2])->header('X-Test', 'present')->retrieve();
        if ($read !== ['body' => null, 'ok' => true]) throw new Exception('RestClient read');
        $posted = $rest->post('/products')->body(['sku' => 'anvil'])->retrieve();
        if ($posted['body'] !== ['sku' => 'anvil']) throw new Exception('RestClient write');
        if (RestHolder::$seen !== ['GET /products', 'POST /products']) throw new Exception('RestClient transport: '.implode(',', RestHolder::$seen));
        self::rejectBehavior('RestClient query path', fn() => $rest->get('/products?page=1'));
        self::rejectBehavior('RestClient traversal', fn() => $rest->get('/../secrets'));
        self::rejectBehavior('RestClient read body', fn() => $rest->get('/products')->body([]));
        self::rejectBehavior('RestClient adapter name', fn() => $context->restClient('not an identifier'));
        self::rejectBehavior('RestClient transport header', fn() => $rest->get('/products')->header('Host', 'attacker.example')->retrieve());
    }

    public static function checkCancellation(BehaviorSignal $signal, BehaviorContext $context, Service $service): void {
        $signal->abort();
        self::rejectBehavior('owner abort', fn() => self::read($context, $service), 'cancelled');
        $owner = new BehaviorSignal();
        $child = new BehaviorSignal();
        $execution = new BehaviorExecution('tenant', 'yon', $child, $owner, 1000);
        $owner->abort();
        if (!$execution->signal->aborted()) self::$failures[] = 'adapter cancellation view does not observe owner abort';
    }

    private static function createShared(string $partition): BehaviorContext {
        return Behaviors::create('NotesSetup', ['partition' => $partition, 'signal' => new BehaviorSignal()]);
    }

    public static function checkSharedSetup(): void {
        $writer = self::createShared('writer');
        $reader = self::createShared('reader');
        if (NotesSetup::$configured !== 1) self::$failures[] = 'two contexts from one Setup configure it more than once';
        $writer->restClient('notes')->post('/notes')->body('Grace')->retrieve();
        $answer = $reader->restClient('notes')->get('/notes')->retrieve();
        if ($answer !== ['items' => ['Ada', 'Grace']]) self::$failures[] = 'two contexts from one Setup do not share its adapters';
        $writer->dispose();
        $writer->dispose();
        $later = self::createShared('later');
        if (NotesSetup::$configured !== 1) self::$failures[] = 'a Setup is configured again while one of its contexts is open';
        $reader->dispose();
        $later->dispose();
        self::createShared('fresh')->dispose();
        if (NotesSetup::$configured !== 2) self::$failures[] = 'a Setup is not configured again after its last context is disposed';
    }

    public static function run(): void {
        self::loadRuntime();
        $entity = ['name' => 'Product', 'entity' => ['adapter' => 'store', 'id' => 'id'], 'table' => 'products', 'methods' => []];
        $signal = new BehaviorSignal();
        $context = self::serviceContext($signal, $entity);
        $service = new Service();
        $service->behaviors = $context;
        self::checkCache($context, $service);
        self::checkHttp($context, $service);
        self::checkRepository($context, $entity);
        self::checkBoundaries();
        self::checkRejections();
        self::checkResilience();
        self::checkTransactions();
        self::checkJoinedTransactions();
        self::checkRestClient();
        self::checkSharedSetup();
        self::checkCancellation($signal, $context, $service);
        if (self::$failures) throw new Exception(implode("\n", self::$failures));
        echo "PHP behavior boundary contracts passed\n";
    }
}

RuntimeContract::run();
