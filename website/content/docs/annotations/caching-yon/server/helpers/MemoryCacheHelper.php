<?php

#[Helper]
class MemoryCacheHelper {
    public static array $entries = [];

    public static function get($key, $execution) {
        return self::$entries[$key] ?? ['hit' => false];
    }

    public static function set($key, $value, $execution): void {
        self::$entries[$key] = ['hit' => true, 'value' => $value, 'expiresAt' => $execution->expiresAt];
    }

    public static function delete($key, $execution): void {
        unset(self::$entries[$key]);
    }

    public static function authorize($call, $execution) {
        return true;
    }
}
