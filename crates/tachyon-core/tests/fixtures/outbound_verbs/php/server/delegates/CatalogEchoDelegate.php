<?php

#[Delegate]
class CatalogEchoDelegate {
    public static function exchange(array $request, mixed $execution): array {
        return ['method' => $request['method'], 'path' => $request['path'], 'query' => $request['query'] ?? null, 'headers' => $request['headers'] ?? null, 'body' => $request['body'] ?? null];
    }
}
