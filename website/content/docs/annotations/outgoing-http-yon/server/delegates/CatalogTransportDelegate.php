<?php

#[Delegate]
class CatalogTransportDelegate {
    public static function exchange(mixed $request, mixed $execution): array { return ['items' => ['anvil']]; }
}
