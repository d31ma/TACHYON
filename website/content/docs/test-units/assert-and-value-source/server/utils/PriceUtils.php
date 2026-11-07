<?php
final class PriceUtils {
    public static function withTax(int $cents): int {
        self::requirePositive($cents);
        return (int) round($cents * 1.2);
    }

    private static function requirePositive(int $cents): void {
        if ($cents < 0) {
            throw new InvalidArgumentException('A price is never negative.');
        }
    }
}
