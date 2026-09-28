<?php
final class Middleware
{
    public static function before(): void
    {
        $path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
        if (str_starts_with($path, '/private') && ($_SERVER['HTTP_AUTHORIZATION'] ?? '') !== 'let-me-in') {
            http_response_code(401);
            header('Content-Type: application/json');
            echo '{"error":"Unauthorized"}';
        }
    }

    public static function after(): void
    {
        header('x-served-by: tachyon');
    }
}
