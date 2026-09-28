<?php

#[Helper]
class MemoryCacheHelper {
    public static array $entries = [];

    public static function get(string $key, mixed $execution): array {
        return self::$entries[$key] ?? ['hit' => false];
    }

    public static function set(string $key, mixed $value, mixed $execution): void {
        self::$entries[$key] = ['hit' => true, 'value' => $value, 'expiresAt' => $execution->expiresAt];
    }

    public static function delete(string $key, mixed $execution): void {
        unset(self::$entries[$key]);
    }

    public static function authorize(mixed $call, mixed $execution): bool {
        return true;
    }
}
