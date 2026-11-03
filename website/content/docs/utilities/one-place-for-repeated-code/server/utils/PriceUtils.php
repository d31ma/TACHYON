<?php
final class PriceUtils {
    public static function label(int $cents): string {
        return '$' . self::dollars($cents);
    }

    private static function dollars(int $cents): string {
        return number_format($cents / 100, 2, '.', '');
    }
}
