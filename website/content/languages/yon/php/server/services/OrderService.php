<?php

#[Service]
class OrderService
{
    public static function list(): array
    {
        return OrderStore::all();
    }
}
