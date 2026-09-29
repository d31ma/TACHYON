final class Assert {
    private const NOTHING_THROWN = 'Expected an error to be thrown, but nothing was thrown.';
    private const MESSAGE_SEPARATOR = ' ==> ';

    public static function equals(mixed $expected, mixed $actual, ?string $message = null): void {
        $failure = 'expected: <' . self::show($expected) . '> but was: <' . self::show($actual) . '>';
        self::check($expected == $actual, $failure, $message);
    }

    public static function isTrue(mixed $condition, ?string $message = null): void {
        $failure = 'expected: <true> but was: <' . self::show($condition) . '>';
        self::check($condition === true, $failure, $message);
    }

    public static function throws(callable $action, ?string $message = null): \Throwable {
        try {
            $action();
        } catch (\Throwable $error) {
            return $error;
        }
        throw self::failure(self::NOTHING_THROWN, $message);
    }

    private static function check(bool $passed, string $failure, ?string $message): void {
        if (!$passed) {
            throw self::failure($failure, $message);
        }
    }

    private static function failure(string $failure, ?string $message): \AssertionError {
        if ($message === null) {
            return new \AssertionError($failure);
        }
        return new \AssertionError($message . self::MESSAGE_SEPARATOR . $failure);
    }

    private static function show(mixed $value): string {
        if (is_string($value)) {
            return $value;
        }
        $encoded = json_encode($value);
        if ($encoded === false) {
            return var_export($value, true);
        }
        return $encoded;
    }
}
