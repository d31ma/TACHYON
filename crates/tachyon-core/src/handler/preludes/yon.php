<?php

final class DataSource
{
    public function query(string $sql, array $parameters): mixed
    {
        return YonManagedResources::call(['operation' => 'sql', 'sql' => $sql, 'parameters' => $parameters, 'mode' => 'rows']);
    }
    public function execute(string|array $sql, array $parameters = []): mixed
    {
        return is_array($sql) ? $this->command($sql) : YonManagedResources::call(['operation' => 'sql', 'sql' => $sql, 'parameters' => $parameters, 'mode' => 'execute']);
    }
    public function command(array $command): mixed { return YonManagedResources::call($command); }
    public function findById(string $collection, string $id): mixed { return self::document('findById', ['collection' => $collection, 'id' => $id]); }
    public function insert(string $collection, array $document): mixed { return self::document('insert', ['collection' => $collection, 'value' => $document]); }
    public function patch(string $collection, string $id, array $fields): mixed
    {
        return self::document('patch', ['collection' => $collection, 'id' => $id, 'value' => $fields]);
    }
    public function deleteById(string $collection, string $id): mixed { return self::document('deleteById', ['collection' => $collection, 'id' => $id]); }
    public function scroll(string $collection, ?array $where = null, ?string $cursor = null, int $size = 20): mixed
    {
        $command = ['operation' => 'document', 'command' => 'scroll', 'collection' => $collection, 'where' => $where, 'cursor' => $cursor, 'size' => $size];
        return YonManagedResources::call($command, YonManagedResources::SCROLL_TIMEOUT_NANOSECONDS);
    }
    public function createCollection(string $name): mixed { return self::document('createCollection', ['collection' => $name]); }
    public function dropCollection(string $name): mixed { return self::document('dropCollection', ['collection' => $name]); }
    public function collectionExists(string $name): mixed { return self::document('collectionExists', ['collection' => $name]); }
    public function publish(string $topic, mixed $payload, ?string $key = null): mixed
    {
        return self::queue('publish', ['topic' => $topic, 'payload' => $payload, 'key' => $key]);
    }
    public function claim(string $topic, string $group, int $maxMessages = 1, int $leaseMs = 30000): mixed
    {
        return self::queue('claim', ['topic' => $topic, 'group' => $group, 'maxMessages' => $maxMessages, 'leaseMs' => $leaseMs]);
    }
    public function ack(string $topic, string $group, array $delivery): mixed
    {
        return self::queue('ack', ['topic' => $topic, 'group' => $group, 'id' => $delivery['id'], 'receipt' => $delivery['receipt']]);
    }
    public function nack(string $topic, string $group, array $delivery, int $delayMs = 0): mixed
    {
        return self::queue('nack', ['topic' => $topic, 'group' => $group, 'id' => $delivery['id'], 'receipt' => $delivery['receipt'], 'delayMs' => $delayMs]);
    }
    private static function document(string $command, array $fields): mixed
    {
        return YonManagedResources::call(['operation' => 'document', 'command' => $command] + $fields);
    }
    private static function queue(string $command, array $fields): mixed
    {
        return YonManagedResources::call(['operation' => 'queue', 'command' => $command] + $fields);
    }
}

final class RedisTemplate
{
    public function get(string $key): mixed { return YonManagedResources::call(['operation' => 'redis', 'command' => 'get', 'key' => $key]); }
    public function delete(string $key): mixed { return YonManagedResources::call(['operation' => 'redis', 'command' => 'delete', 'key' => $key]); }
    public function set(string $key, mixed $value, int $ttlSeconds): mixed
    {
        return YonManagedResources::call(['operation' => 'redis', 'command' => 'set', 'key' => $key, 'value' => $value, 'ttlSeconds' => $ttlSeconds]);
    }
}

final class YonManagedResources
{
    private const LIMIT = 1024 * 1024;
    public const SCROLL_TIMEOUT_NANOSECONDS = 30_000_000_000;
    private static function failed(): never { throw new RuntimeException('Yon managed resource operation failed.'); }
    public static function call(mixed $command, int $timeoutNanoseconds = 5_000_000_000): mixed
    {
        try {
            $body = json_encode($command, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (Throwable $ignored) {
            self::failed();
        }
        return self::exchange($body, $timeoutNanoseconds);
    }
    public static function exchange(string $body, int $timeoutNanoseconds): mixed
    {
        $socket = false;
        try {
            $endpoint = getenv('YON_RESOURCE_ENDPOINT');
            $token = getenv('YON_RESOURCE_TOKEN');
            if (!is_string($endpoint) || preg_match('~\Ahttp://127\.0\.0\.1:([0-9]{1,5})/\z~', $endpoint, $matches) !== 1 || !is_string($token) || preg_match('/\A[0-9a-f]{64}\z/', $token) !== 1) self::failed();
            $port = (int) $matches[1];
            if ($port < 1 || $port > 65535) self::failed();
            if (strlen($body) > self::LIMIT) self::failed();
            $deadline = hrtime(true) + $timeoutNanoseconds;
            $socket = @stream_socket_client("tcp://127.0.0.1:$port", $errorCode, $errorText, 5, STREAM_CLIENT_CONNECT);
            if (!is_resource($socket)) self::failed();
            $request = "POST / HTTP/1.1\r\nHost: 127.0.0.1:$port\r\nAuthorization: Bearer $token\r\nContent-Type: application/json\r\nContent-Length: " . strlen($body) . "\r\nConnection: close\r\n\r\n" . $body;
            $written = 0;
            while ($written < strlen($request)) {
                self::timeout($socket, $deadline);
                $count = @fwrite($socket, substr($request, $written));
                if ($count === false || $count === 0) self::failed();
                $written += $count;
            }
            $head = '';
            while (!str_ends_with($head, "\r\n\r\n")) {
                self::timeout($socket, $deadline);
                $next = @fread($socket, 1);
                if ($next === false || $next === '' || strlen($head) >= 16384) self::failed();
                $head .= $next;
            }
            $lines = explode("\r\n", $head);
            if (preg_match('~\AHTTP/1\.[01] 200(?: .*)?\z~', $lines[0]) !== 1) self::failed();
            $length = -1;
            foreach (array_slice($lines, 1) as $line) {
                $line = strtolower($line);
                if (str_starts_with($line, 'transfer-encoding:')) self::failed();
                if (str_starts_with($line, 'content-length:')) {
                    $value = trim(substr($line, 15));
                    if ($length >= 0 || preg_match('/\A[0-9]{1,7}\z/', $value) !== 1) self::failed();
                    $length = (int) $value;
                }
            }
            if ($length < 0 || $length > self::LIMIT) self::failed();
            $response = '';
            while (strlen($response) < $length) {
                self::timeout($socket, $deadline);
                $chunk = @fread($socket, $length - strlen($response));
                if ($chunk === false || $chunk === '') self::failed();
                $response .= $chunk;
            }
            return json_decode($response, true, 64, JSON_THROW_ON_ERROR);
        } catch (Throwable $ignored) {
            self::failed();
        } finally {
            if (is_resource($socket)) fclose($socket);
        }
    }
    private static function timeout(mixed $socket, int $deadline): void
    {
        $remaining = $deadline - hrtime(true);
        if ($remaining <= 0) self::failed();
        if (!stream_set_timeout($socket, intdiv($remaining, 1_000_000_000), intdiv($remaining % 1_000_000_000, 1000))) self::failed();
    }
}

final class YonGateway
{
    private const LIMIT = 1024 * 1024;
    private const ANSWER_MARGIN_NANOSECONDS = 1_000_000_000;
    public static function call(string $key, array $arguments): array
    {
        try {
            $body = json_encode(['operation' => 'gateway', 'key' => $key, 'arguments' => array_values($arguments)], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (Throwable $ignored) {
            return ['error' => 'invalid'];
        }
        if (strlen($body) > self::LIMIT) return ['error' => 'invalid'];
        $timeLeft = YonRelayContext::timeLeftNanoseconds();
        if ($timeLeft <= 0) return ['error' => 'timeout'];
        try {
            $answer = YonManagedResources::exchange($body, $timeLeft + self::ANSWER_MARGIN_NANOSECONDS);
        } catch (Throwable $ignored) {
            if (YonRelayContext::timeLeftNanoseconds() <= 0) return ['error' => 'timeout'];
            return ['error' => 'adapter'];
        }
        if (!is_array($answer)) return ['error' => 'adapter'];
        if (array_key_exists('result', $answer) || is_string($answer['error'] ?? null)) return $answer;
        return ['error' => 'adapter'];
    }
}

final class __YonFrame
{
    public function __construct(private array $raw) {}

    public function method(): string
    {
        return (string) ($this->raw['method'] ?? '');
    }

    public function route(): string
    {
        return (string) ($this->raw['route'] ?? '');
    }

    public function parameter(string $name): string
    {
        return (string) ($this->raw['parameters'][$name] ?? '');
    }

    public function body(): string
    {
        return (string) ($this->raw['body']['data'] ?? '');
    }

    public function raw(): array
    {
        return $this->raw;
    }

    public function arguments(): array
    {
        $arguments = $this->raw['arguments'] ?? [];
        return is_array($arguments) ? $arguments : [];
    }

    public function headers(): array
    {
        $headers = $this->raw['headers'] ?? [];
        return is_array($headers) ? $headers : [];
    }

    public function target(): string
    {
        return (string) ($this->raw['target'] ?? $this->route());
    }
}

if (!defined('STDIN')) {
    define('STDIN', fopen('php://stdin', 'rb'));
    define('STDOUT', fopen('php://stdout', 'wb'));
    define('STDERR', fopen('php://stderr', 'wb'));
}

final class YonInputStream
{
    public static string $body = '';
    public $context;
    private $inner = null;
    private int $position = 0;

    public function stream_open(string $path, string $mode, int $options, ?string &$opened_path): bool
    {
        if (strtolower($path) === 'php://input') {
            return true;
        }
        stream_wrapper_restore('php');
        $this->inner = @fopen($path, $mode);
        stream_wrapper_unregister('php');
        stream_wrapper_register('php', self::class);
        return $this->inner !== false;
    }

    public function stream_read(int $count): string|false
    {
        if ($this->inner !== null) {
            return fread($this->inner, $count);
        }
        $part = substr(self::$body, $this->position, $count);
        $this->position += strlen($part);
        return $part;
    }

    public function stream_write(string $data): int
    {
        return $this->inner === null ? 0 : (int) fwrite($this->inner, $data);
    }

    public function stream_eof(): bool
    {
        return $this->inner === null ? $this->position >= strlen(self::$body) : feof($this->inner);
    }

    public function stream_tell(): int
    {
        return $this->inner === null ? $this->position : (int) ftell($this->inner);
    }

    public function stream_seek(int $offset, int $whence): bool
    {
        if ($this->inner !== null) {
            return fseek($this->inner, $offset, $whence) === 0;
        }
        $this->position = $whence === SEEK_SET ? $offset : $this->position + $offset;
        return true;
    }

    public function stream_flush(): bool
    {
        return $this->inner === null || fflush($this->inner);
    }

    public function stream_stat(): array|false
    {
        return $this->inner === null ? ['size' => strlen(self::$body)] : fstat($this->inner);
    }

    public function stream_close(): void
    {
        if ($this->inner !== null) {
            fclose($this->inner);
        }
    }
}

final class YonNativeHttp
{
    private static string $method = 'GET';
    private static string $target = '/';
    private static array $requestHeaders = [];
    private static ?array $server = null;

    public static function begin(__YonFrame $frame): void
    {
        self::$server ??= $_SERVER;
        $_SERVER = self::$server;
        if (YonRelayContext::warm()) {
            $_SERVER['REQUEST_TIME_FLOAT'] = microtime(true);
            $_SERVER['REQUEST_TIME'] = (int) $_SERVER['REQUEST_TIME_FLOAT'];
        }
        self::$method = $frame->method();
        self::$target = $frame->target();
        self::$requestHeaders = $frame->headers();
        YonInputStream::$body = $frame->body();
        $query = (string) parse_url(self::$target, PHP_URL_QUERY);
        $_SERVER['REQUEST_METHOD'] = self::$method;
        $_SERVER['REQUEST_URI'] = self::$target;
        $_SERVER['QUERY_STRING'] = $query;
        $_SERVER['SERVER_PROTOCOL'] = 'HTTP/1.1';
        foreach (self::$requestHeaders as $name => $values) {
            $key = strtoupper(str_replace('-', '_', (string) $name));
            $value = implode(', ', (array) $values);
            $_SERVER[in_array($key, ['CONTENT_TYPE', 'CONTENT_LENGTH'], true) ? $key : 'HTTP_' . $key] = $value;
        }
        $headers = array_change_key_case(self::$requestHeaders);
        $_SERVER['REMOTE_ADDR'] = implode(', ', (array) ($headers['x-real-ip'] ?? []));
        parse_str($query, $_GET);
        $_POST = [];
        if (str_starts_with((string) ($_SERVER['CONTENT_TYPE'] ?? ''), 'application/x-www-form-urlencoded')) {
            parse_str(YonInputStream::$body, $_POST);
        }
        $_COOKIE = self::cookies((string) ($_SERVER['HTTP_COOKIE'] ?? ''));
        $_REQUEST = array_merge($_GET, $_POST);
        stream_wrapper_unregister('php');
        stream_wrapper_register('php', YonInputStream::class);
        ob_start();
    }

    public static function end(int $outputLevel): void
    {
        while (ob_get_level() > $outputLevel) {
            ob_end_clean();
        }
        header_remove();
        http_response_code(200);
        YonInputStream::$body = '';
    }

    private static function cookies(string $header): array
    {
        $cookies = [];
        foreach (explode(';', $header) as $pair) {
            [$name, $value] = array_pad(explode('=', trim($pair), 2), 2, '');
            if ($name !== '') {
                $cookies[$name] = urldecode($value);
            }
        }
        return $cookies;
    }

    public static function message(): array
    {
        return [
            'route' => self::$target,
            'method' => self::$method,
            'headers' => self::$requestHeaders,
            'body' => ['encoding' => 'utf8', 'data' => YonInputStream::$body],
        ];
    }

    public static function write(int $status, array $headers, string $body): void
    {
        foreach ($headers as $name => $values) {
            foreach ((array) $values as $value) {
                header($name . ': ' . $value, false);
            }
        }
        http_response_code($status);
        echo $body;
    }

    public static function answer(mixed $result, bool $middleware): mixed
    {
        $body = (string) ob_get_clean();
        $status = (int) http_response_code();
        $lines = headers_list();
        $written = $status !== 200 || $body !== '' || $lines !== [];
        if (!$written && $middleware && $result === null) {
            return __YonAnswer::empty(204);
        }
        if (!$written) {
            return $result;
        }
        $headers = [];
        foreach ($lines as $line) {
            [$name, $value] = array_pad(explode(':', $line, 2), 2, '');
            $headers[strtolower(trim($name))][] = trim($value);
        }
        $code = $status === 200 && $middleware && $body === '' ? 204 : $status;
        return __YonAnswer::json($body)->headers($headers)->status($code);
    }
}

final class YonModels
{
    public static function read(string $class, mixed $value): mixed
    {
        if ($value === null) {
            return null;
        }
        if ($value instanceof \stdClass) {
            $value = get_object_vars($value);
        }
        if (enum_exists($class)) {
            return is_subclass_of($class, \BackedEnum::class) ? $class::from($value) : constant($class . '::' . $value);
        }
        if (!class_exists($class) || !is_array($value)) {
            return $value;
        }
        $reflection = new \ReflectionClass($class);
        $constructor = $reflection->getConstructor();
        if ($constructor !== null && $constructor->getNumberOfParameters() > 0) {
            return $reflection->newInstanceArgs(self::constructorArguments($reflection, $constructor, $value));
        }
        $model = $reflection->newInstanceWithoutConstructor();
        foreach ($reflection->getProperties(\ReflectionProperty::IS_PUBLIC) as $property) {
            $name = $property->getName();
            if (array_key_exists($name, $value)) {
                $property->setValue($model, self::typed($reflection, $name, $property->getType(), $value[$name]));
            }
        }
        return $model;
    }

    public static function readList(string $class, mixed $value): mixed
    {
        if (!is_array($value)) {
            return $value;
        }
        $items = [];
        foreach ($value as $item) {
            $items[] = self::read($class, $item);
        }
        return $items;
    }

    private static function constructorArguments(\ReflectionClass $reflection, \ReflectionMethod $constructor, array $value): array
    {
        $arguments = [];
        foreach ($constructor->getParameters() as $parameter) {
            $name = $parameter->getName();
            if (array_key_exists($name, $value)) {
                $arguments[$name] = self::typed($reflection, $name, $parameter->getType(), $value[$name]);
            }
        }
        return $arguments;
    }

    private static function typed(\ReflectionClass $owner, string $name, ?\ReflectionType $type, mixed $value): mixed
    {
        if (!$type instanceof \ReflectionNamedType) {
            return $value;
        }
        if ($type->getName() === 'array') {
            $value = $value instanceof \stdClass ? get_object_vars($value) : $value;
            $item = self::itemClass($owner, $name);
            return $item === '' ? $value : self::readList($item, $value);
        }
        if ($type->isBuiltin()) {
            return $value;
        }
        return self::read($type->getName(), $value);
    }

    private static function itemClass(\ReflectionClass $owner, string $name): string
    {
        $comment = $owner->hasProperty($name) ? $owner->getProperty($name)->getDocComment() : false;
        $pattern = '/@var\s+\??(?:list<|array<|non-empty-list<)?\\\\?([A-Za-z_][A-Za-z0-9_\\\\]*)(?:>|\[\])/';
        if ($comment === false || preg_match($pattern, $comment, $match) !== 1) {
            return '';
        }
        $local = $owner->getNamespaceName() . '\\' . $match[1];
        return class_exists($local) || enum_exists($local) ? $local : $match[1];
    }
}

final class __YonAnswer
{
    private function __construct(
        private int $status,
        private array $headers,
        private string $body,
    ) {}

    public static function json(string $body): self
    {
        return new self(200, ['content-type' => ['application/json']], $body);
    }

    public static function empty(int $status): self
    {
        return new self($status, [], '');
    }

    public function status(int $status): self
    {
        $this->status = $status;
        return $this;
    }

    public function headers(array $headers): self
    {
        $this->headers = $headers;
        return $this;
    }

    public function header(string $name, string $value): self
    {
        $this->headers[$name][] = $value;
        return $this;
    }

    public function envelope(): string
    {
        return json_encode([
            'status' => $this->status,
            'headers' => (object) $this->headers,
            'body' => $this->body,
        ]);
    }
}

final class YonRelayContext
{
    private const DEFAULT_DEADLINE_MILLISECONDS = 30000;
    private static ?int $startedAt = null;
    private static array $environment = [];
    private static ?int $deadline = null;
    private static string $requestId = 'unknown';

    public static function captureStart(): void
    {
        if (self::$startedAt !== null) {
            return;
        }
        self::$startedAt = hrtime(true);
        self::$environment = self::relayEnvironment(getenv());
    }

    public static function readInvocation(): __YonFrame
    {
        $raw = YonSocket::framed() ? YonSocket::readFrame() : stream_get_contents(STDIN);
        $request = new __YonFrame(json_decode($raw ?: '{}', true) ?? []);
        self::captureInvocation($request->raw());
        return $request;
    }

    public static function warm(): bool
    {
        return getenv('YON_HANDLER_WARM') === '1' && !YonSocket::framed();
    }

    public static function readWarmInvocation(): ?__YonFrame
    {
        $raw = YonSocket::readRequestFrame();
        if ($raw === null) {
            return null;
        }
        self::$startedAt = hrtime(true);
        self::forgetInvocation();
        $fields = json_decode($raw, true);
        if (!is_array($fields) || ($fields['kind'] ?? null) !== 'request') {
            exit(70);
        }
        self::useResources($fields['resources'] ?? null);
        unset($fields['resources']);
        self::captureInvocation($fields);
        return new __YonFrame($fields);
    }

    public static function forgetInvocation(): void
    {
        self::$deadline = null;
        self::$requestId = 'unknown';
        self::useResources(null);
    }

    private static function useResources(mixed $resources): void
    {
        putenv('YON_RESOURCE_ENDPOINT');
        putenv('YON_RESOURCE_TOKEN');
        $endpoint = is_array($resources) ? ($resources['endpoint'] ?? null) : null;
        $token = is_array($resources) ? ($resources['token'] ?? null) : null;
        if (!is_string($endpoint) || !is_string($token)) {
            return;
        }
        putenv('YON_RESOURCE_ENDPOINT=' . $endpoint);
        putenv('YON_RESOURCE_TOKEN=' . $token);
    }

    private static function captureInvocation(array $raw): void
    {
        if (self::$deadline !== null || self::$startedAt === null) {
            return;
        }
        $id = $raw['request_id'] ?? null;
        if (is_string($id)) {
            self::$requestId = $id;
        }
        self::$deadline = self::$startedAt + (int) (self::deadlineMilliseconds($raw) * 1000000);
    }

    public static function renewDeadline(): void
    {
        self::$deadline = hrtime(true) + self::DEFAULT_DEADLINE_MILLISECONDS * 1000000;
    }

    public static function timeLeftNanoseconds(): int
    {
        if (self::$deadline === null) {
            return 0;
        }
        return self::$deadline - hrtime(true);
    }

    public static function environment(): array
    {
        return self::$environment;
    }

    public static function requestId(): string
    {
        return self::$requestId;
    }

    private static function relayEnvironment(array $environment): array
    {
        $kept = [];
        foreach ($environment as $name => $value) {
            $upper = strtoupper((string) $name);
            if (in_array($upper, ['YON_APPLICATION_JSON', 'YON_HANDLER_WARM'], true) || str_starts_with($upper, 'YON_RESOURCE_')) {
                continue;
            }
            $kept[$name] = $value;
        }
        return $kept;
    }

    private static function deadlineMilliseconds(array $raw): float
    {
        $milliseconds = $raw['deadline_ms'] ?? null;
        if (!is_int($milliseconds) && !is_float($milliseconds)) {
            return self::DEFAULT_DEADLINE_MILLISECONDS;
        }
        if (!is_finite($milliseconds)) {
            return self::DEFAULT_DEADLINE_MILLISECONDS;
        }
        return $milliseconds;
    }
}

final class YonRelayExchange
{
    private const MAX_STDOUT_BYTES = 16 * 1024 * 1024;
    private const MAX_STDERR_BYTES = 64 * 1024;
    private const LONGEST_WAIT_MICROSECONDS = 100000;
    private const EXIT_POLL_MICROSECONDS = 1000;
    private const CHUNK_BYTES = 8192;
    public string $stdout = '';
    public bool $stdoutOverflow = false;
    public bool $stderrOverflow = false;
    private int $written = 0;
    private int $stderrBytes = 0;

    public function __construct(private array $pipes, private string $payload)
    {
        foreach ($this->pipes as $pipe) {
            stream_set_blocking($pipe, false);
        }
    }

    public function finishedInTime(mixed $process): bool
    {
        while (YonRelayContext::timeLeftNanoseconds() > 0) {
            $read = $this->openOutputs();
            $write = $this->pendingInput();
            if ($read === [] && $write === []) {
                if (!proc_get_status($process)['running']) {
                    return true;
                }
                usleep(self::EXIT_POLL_MICROSECONDS);
                continue;
            }
            $except = [];
            if (@stream_select($read, $write, $except, 0, $this->waitMicroseconds()) === false) {
                return false;
            }
            $this->writeInput($write);
            $this->readOutputs($read);
        }
        return false;
    }

    public function closePipes(): void
    {
        foreach ($this->pipes as $pipe) {
            if (is_resource($pipe)) {
                fclose($pipe);
            }
        }
    }

    private function openOutputs(): array
    {
        $open = [];
        foreach ([1, 2] as $number) {
            $pipe = $this->pipes[$number] ?? null;
            if (is_resource($pipe) && !feof($pipe)) {
                $open[] = $pipe;
            }
        }
        return $open;
    }

    private function pendingInput(): array
    {
        $input = $this->pipes[0] ?? null;
        if (!is_resource($input) || $this->written >= strlen($this->payload)) {
            return [];
        }
        return [$input];
    }

    private function waitMicroseconds(): int
    {
        $left = intdiv(max(0, YonRelayContext::timeLeftNanoseconds()), 1000);
        return min(self::LONGEST_WAIT_MICROSECONDS, $left);
    }

    private function writeInput(array $ready): void
    {
        if ($ready === []) {
            return;
        }
        $count = @fwrite($this->pipes[0], substr($this->payload, $this->written, self::CHUNK_BYTES));
        if ($count === false) {
            $this->closeInput();
            return;
        }
        $this->written += $count;
        if ($this->written >= strlen($this->payload)) {
            $this->closeInput();
        }
    }

    private function closeInput(): void
    {
        fclose($this->pipes[0]);
        $this->pipes[0] = null;
    }

    private function readOutputs(array $ready): void
    {
        foreach ($ready as $pipe) {
            $chunk = @fread($pipe, self::CHUNK_BYTES);
            if ($chunk === false || $chunk === '') {
                continue;
            }
            if ($pipe === $this->pipes[1]) {
                $this->keepStdout($chunk);
            } else {
                $this->countStderr($chunk);
            }
        }
    }

    private function keepStdout(string $chunk): void
    {
        $remaining = self::MAX_STDOUT_BYTES - strlen($this->stdout);
        if (strlen($chunk) > $remaining) {
            $this->stdoutOverflow = true;
        }
        if ($remaining > 0) {
            $this->stdout .= substr($chunk, 0, $remaining);
        }
    }

    private function countStderr(string $chunk): void
    {
        $this->stderrBytes += strlen($chunk);
        if ($this->stderrBytes > self::MAX_STDERR_BYTES) {
            $this->stderrOverflow = true;
        }
    }
}

final class Yon
{
    private const FAILURE_MESSAGE = 'Component invocation failed.';
    private const FALLBACK_HIGHEST_DESCRIPTOR = 255;

    public static function relay(array $command): void
    {
        $answer = json_decode(self::relayAnswer($command)->envelope(), true);
        YonNativeHttp::write((int) $answer['status'], (array) $answer['headers'], (string) $answer['body']);
    }

    private static function relayAnswer(array $command): __YonAnswer
    {
        if ($command === []) {
            return self::relayFailed('start');
        }
        if (YonRelayContext::timeLeftNanoseconds() <= 0) {
            return self::relayFailed('timeout');
        }
        $process = @proc_open($command, self::descriptors(), $pipes, null, YonRelayContext::environment());
        if (!is_resource($process)) {
            return self::relayFailed('start');
        }
        $exchange = new YonRelayExchange($pipes, json_encode(YonNativeHttp::message()) ?: '{}');
        $finished = $exchange->finishedInTime($process);
        if (!$finished) {
            @proc_terminate($process, 9);
        }
        $exchange->closePipes();
        $status = @proc_close($process);
        if (!$finished) {
            return self::relayFailed('timeout');
        }
        if ($exchange->stdoutOverflow || $exchange->stderrOverflow) {
            return self::relayFailed('overflow');
        }
        if ($status !== 0) {
            return self::relayFailed('exit');
        }
        return self::responseFromOutput($exchange->stdout);
    }

    private static function descriptors(): array
    {
        if (PHP_OS_FAMILY === 'Windows') {
            return [0 => ['socket'], 1 => ['socket'], 2 => ['file', 'NUL', 'a']];
        }
        $descriptors = [0 => ['pipe', 'r'], 1 => ['pipe', 'w'], 2 => ['pipe', 'w']];
        foreach (self::openDescriptorsAboveStandard() as $number) {
            $descriptors[$number] = ['file', '/dev/null', 'r'];
        }
        return $descriptors;
    }

    private static function openDescriptorsAboveStandard(): array
    {
        foreach (['/proc/self/fd', '/dev/fd'] as $directory) {
            $names = @scandir($directory);
            if ($names === false) {
                continue;
            }
            $numbers = [];
            foreach ($names as $name) {
                if (ctype_digit($name) && (int) $name > 2) {
                    $numbers[] = (int) $name;
                }
            }
            return $numbers;
        }
        return range(3, self::FALLBACK_HIGHEST_DESCRIPTOR);
    }

    private static function responseFromOutput(string $stdout): __YonAnswer
    {
        $decoded = json_decode($stdout);
        if (!($decoded instanceof stdClass)) {
            return self::relayFailed('protocol');
        }
        $envelope = get_object_vars($decoded);
        $response = __YonAnswer::json(self::text($envelope['body'] ?? ''))
            ->status(is_int($envelope['status'] ?? null) ? $envelope['status'] : 200);
        $headers = get_object_vars(($envelope['headers'] ?? null) instanceof stdClass ? $envelope['headers'] : new stdClass());
        if ($headers !== []) {
            $response = $response->headers([]);
        }
        foreach ($headers as $name => $values) {
            foreach (is_array($values) ? $values : [$values] as $value) {
                $response = $response->header((string) $name, self::text($value));
            }
        }
        return $response;
    }

    private static function text(mixed $value): string
    {
        if (is_array($value) || is_object($value)) {
            return json_encode($value) ?: '';
        }
        return (string) $value;
    }

    private static function relayFailed(string $category): __YonAnswer
    {
        fwrite(STDERR, json_encode([
            'event' => 'handler.relay_failed',
            'request_id' => YonRelayContext::requestId(),
            'category' => $category,
        ]) . "\n");
        return __YonAnswer::json(json_encode(['error' => self::FAILURE_MESSAGE]))->status(502);
    }
}

final class YonStream
{
    private const MAX_FRAME_BYTES = 16 * 1024 * 1024;

    public static function event(string $requestId, mixed $value): bool
    {
        $data = json_encode($value, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($data === false) {
            return self::failure($requestId, 'Streamed event is not JSON-encodable.');
        }
        return self::frame($requestId, [
            'protocol_version' => 1,
            'kind' => 'event',
            'request_id' => $requestId,
            'body' => ['encoding' => 'utf8', 'data' => $data],
        ]);
    }

    public static function failure(string $requestId, string $message): bool
    {
        self::frame($requestId, [
            'protocol_version' => 1,
            'kind' => 'response',
            'request_id' => $requestId,
            'status' => 500,
            'headers' => (object) [],
            'error' => ['code' => 'TY2204', 'message' => $message, 'retryable' => false],
        ]);
        return false;
    }

    public static function end(string $requestId): void
    {
        self::frame($requestId, ['protocol_version' => 1, 'kind' => 'end', 'request_id' => $requestId]);
    }

    private static function frame(string $requestId, array $frame): bool
    {
        $payload = json_encode($frame);
        if ($payload === false || strlen($payload) > self::MAX_FRAME_BYTES) {
            $payload = json_encode([
                'protocol_version' => 1,
                'kind' => 'response',
                'request_id' => $requestId,
                'status' => 500,
                'headers' => (object) [],
                'error' => [
                    'code' => 'TY2203',
                    'message' => 'Streamed event exceeds the protocol frame limit.',
                    'retryable' => false,
                ],
            ]);
            fwrite(STDOUT, pack('N', strlen($payload)) . $payload);
            fflush(STDOUT);
            return false;
        }
        fwrite(STDOUT, pack('N', strlen($payload)) . $payload);
        fflush(STDOUT);
        return true;
    }
}

interface YonSocketEndpoint
{
    public function open(SocketSession $session): void;
    public function message(SocketSession $session, mixed $value): void;
    public function close(SocketSession $session): void;
    public function error(SocketSession $session, Throwable $error): void;
}

final class SocketPeer
{
    public function __construct(private SocketSession $session, public readonly string $id) {}

    public function send(mixed $message): void
    {
        $this->session->sendTo($message, $this->id);
    }
}

final class SocketSession
{
    private array $inbox = [];
    private bool $closed = false;
    private ?array $sessions = null;

    public function __construct(public readonly string $id) {}

    public function send(mixed $message): void
    {
        $this->sendTo($message, null);
    }

    public function close(): void
    {
        $this->closed = true;
        $this->write(['kind' => 'close']);
    }

    public function openSessions(): array
    {
        $this->sessions = null;
        $this->write(['kind' => 'sessions']);
        while ($this->sessions === null && !$this->closed) {
            $this->receive();
        }
        return array_map(fn ($id) => new SocketPeer($this, (string) $id), $this->sessions ?? []);
    }

    public function sendTo(mixed $message, ?string $to): void
    {
        $data = json_encode($message, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        $frame = ['kind' => 'event', 'body' => ['encoding' => 'utf8', 'data' => $data]];
        if ($to !== null) {
            $frame['to'] = $to;
        }
        $this->write($frame);
    }

    public function next(): ?string
    {
        while ($this->inbox === [] && !$this->closed) {
            $this->receive();
        }
        return array_shift($this->inbox);
    }

    private function receive(): void
    {
        $raw = YonSocket::readFrame();
        $frame = $raw === null ? null : json_decode($raw, true);
        if (!is_array($frame)) {
            $this->closed = true;
            return;
        }
        if (($frame['request_id'] ?? null) !== $this->id) {
            exit(70);
        }
        match ($frame['kind'] ?? null) {
            'message' => $this->inbox[] = (string) ($frame['body']['data'] ?? 'null'),
            'sessions' => $this->sessions = is_array($frame['sessions'] ?? null) ? $frame['sessions'] : [],
            default => $this->closed = true,
        };
    }

    private function write(array $frame): void
    {
        $frame['protocol_version'] = 1;
        $frame['request_id'] = $this->id;
        YonSocket::writeFrame(json_encode($frame, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
    }
}

final class YonSocket
{
    private const MAX_FRAME_BYTES = 16 * 1024 * 1024;

    public static function framed(): bool
    {
        return getenv('YON_SOCKET_FRAMES') === '1';
    }

    public static function readFrame(): ?string
    {
        $prefix = self::readExactly(4);
        if ($prefix === null) {
            return null;
        }
        $length = unpack('N', $prefix)[1];
        if ($length > self::MAX_FRAME_BYTES) {
            exit(70);
        }
        return $length === 0 ? '' : self::readExactly($length);
    }

    public static function readRequestFrame(): ?string
    {
        $first = fread(STDIN, 1);
        if ($first === false || $first === '') {
            return null;
        }
        $rest = self::readExactly(3);
        if ($rest === null) {
            exit(70);
        }
        $length = unpack('N', $first . $rest)[1];
        if ($length > self::MAX_FRAME_BYTES) {
            exit(70);
        }
        $payload = $length === 0 ? '' : self::readExactly($length);
        if ($payload === null) {
            exit(70);
        }
        return $payload;
    }

    private static function readExactly(int $length): ?string
    {
        $text = '';
        while (strlen($text) < $length) {
            $part = fread(STDIN, $length - strlen($text));
            if ($part === false || $part === '') {
                return null;
            }
            $text .= $part;
        }
        return $text;
    }

    public static function writeFrame(string $text): void
    {
        fwrite(STDOUT, pack('N', strlen($text)) . $text);
        fflush(STDOUT);
    }

    public static function run(YonSocketEndpoint $endpoint, __YonFrame $request): never
    {
        $session = new SocketSession((string) ($request->raw()['request_id'] ?? ''));
        self::attempt($endpoint, $session, fn () => $endpoint->open($session));
        while (($message = $session->next()) !== null) {
            self::attempt($endpoint, $session, fn () => $endpoint->message($session, json_decode($message, true)));
        }
        try {
            YonRelayContext::renewDeadline();
            $endpoint->close($session);
        } catch (Throwable) {
        }
        exit(0);
    }

    private static function attempt(YonSocketEndpoint $endpoint, SocketSession $session, callable $action): void
    {
        try {
            YonRelayContext::renewDeadline();
            $action();
        } catch (Throwable $failure) {
            fwrite(STDERR, json_encode([
                'event' => 'handler.socket_failed',
                'request_id' => $session->id,
                'message' => $failure->getMessage(),
            ]) . "\n");
            try {
                YonRelayContext::renewDeadline();
                $endpoint->error($session, $failure);
            } catch (Throwable) {
            }
        }
    }
}

final class YonMain
{
    private const MAX_CAUSE_DEPTH = 8;
    private const CHECK_REJECTED_STATUS = 422;

    public static function run(): void
    {
        YonRelayContext::captureStart();
        if (YonRelayContext::warm()) {
            self::serveWarm();
        }
        echo self::respond(YonRelayContext::readInvocation());
    }

    private static function serveWarm(): never
    {
        register_shutdown_function(static function (): void {
            while (ob_get_level() > 0) {
                ob_end_clean();
            }
        });
        $outputLevel = ob_get_level();
        YonSocket::writeFrame('{"protocol_version":1,"kind":"ready"}');
        while (($request = YonRelayContext::readWarmInvocation()) !== null) {
            $answer = self::respond($request);
            if ($answer !== null) {
                YonSocket::writeFrame($answer);
            }
            YonNativeHttp::end($outputLevel);
            YonRelayContext::forgetInvocation();
        }
        exit(0);
    }

    private static function respond(__YonFrame $request): ?string
    {
        YonNativeHttp::begin($request);
        try {
            $response = self::invoke($request);
        } catch (Throwable $failure) {
            $violations = self::violations($failure);
            if ($violations === null) {
                throw $failure;
            }
            ob_end_clean();
            return json_encode(['status' => self::CHECK_REJECTED_STATUS, 'violations' => $violations], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        }
        if ($response instanceof Generator) {
            self::stream($request, $response);
            return null;
        }
        $response = YonNativeHttp::answer($response, self::phase($request) !== null);
        if (!($response instanceof __YonAnswer)) {
            $response = __YonAnswer::json(json_encode(
                $response, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE
            ));
        }
        return $response->envelope();
    }

    private static function violations(Throwable $failure): ?array
    {
        $cause = $failure;
        for ($depth = 0; $cause !== null && $depth < self::MAX_CAUSE_DEPTH; $depth++) {
            if ($cause instanceof BehaviorRejection) {
                return $cause->violations;
            }
            $cause = $cause->getPrevious();
        }
        return null;
    }

    private static function phase(__YonFrame $request): ?string
    {
        return match ((string) ($request->raw()['operation'] ?? '')) {
            'middleware.before' => 'before',
            'middleware.after' => 'after',
            default => null,
        };
    }

    private static function invoke(__YonFrame $request): mixed
    {
        $phase = self::phase($request);
        if ($phase !== null) {
            return method_exists('__YON_HANDLER__', $phase) ? __YON_HANDLER__::{$phase}() : null;
        }
        if (($request->raw()['operation'] ?? '') === 'event.deliver') {
            return method_exists('__YON_HANDLER__', 'yonDeliver') ? __YON_HANDLER__::yonDeliver($request) : __YonAnswer::empty(404);
        }
        $method = $request->method();
        if (!method_exists('__YON_HANDLER__', $method)) {
            return __YonAnswer::empty(405);
        }
        if (defined('__YON_HANDLER__::YON_FRAMES')) {
            return __YON_HANDLER__::{$method}($request);
        }
        return __YON_HANDLER__::{$method}();
    }

    private static function stream(__YonFrame $request, Generator $events): void
    {
        $id = (string) ($request->raw()['request_id'] ?? '');
        try {
            foreach ($events as $event) {
                if (!YonStream::event($id, $event)) {
                    exit(0);
                }
            }
        } catch (Throwable) {
            YonStream::failure($id, 'The handler stream failed.');
            exit(0);
        }
        if (!YonRelayContext::warm()) {
            exit(0);
        }
        YonStream::end($id);
    }
}

YonRelayContext::captureStart();
