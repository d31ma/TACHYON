<?php
class OrderService
{
    public static function place(string $item): array
    {
        return ['placed' => $item];
    }
}
