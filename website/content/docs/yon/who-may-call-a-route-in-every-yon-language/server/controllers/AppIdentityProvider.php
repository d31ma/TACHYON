<?php
#[IdentityProvider]
class AppIdentityProvider {
    private const SESSIONS = [
        'ada-token' => ['user' => 'ada', 'roles' => ['Admin']],
        'bob-token' => ['user' => 'bob', 'roles' => []],
    ];

    public function identify(): ?array {
        return $this->session($this->token($_SERVER));
    }

    /** @param array<string, mixed> $server */
    private function token(array $server): string {
        return strval($server['HTTP_AUTHORIZATION'] ?? '');
    }

    private function session(string $token): ?array {
        return self::SESSIONS[$token] ?? null;
    }
}
