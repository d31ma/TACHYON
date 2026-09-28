<?php

#[Helper]
class CatalogTransportHelper {
    public static function exchange(mixed $request, mixed $execution): array { return ['items' => ['anvil']]; }
}
