<?php
#[IdentityProvider]
class AppIdentityProvider {
    private const SESSIONS = [
        'ada-token' => ['user' => 'ada', 'roles' => ['Admin']],
        'bob-token' => ['user' => 'bob', 'roles' => []],
    ];
    private const BROKEN_TOKEN = 'broken-token';

    public function identify(): ?array {
        $token = $this->token($_SERVER);
        $this->refuseBroken($token);
        return $this->session($token);
    }

    /** @param array<string, mixed> $server */
    private function token(array $server): string {
        return strval($server['HTTP_AUTHORIZATION'] ?? '');
    }

    private function refuseBroken(string $token): void {
        if ($token === self::BROKEN_TOKEN) {
            throw new RuntimeException('the session store is down');
        }
    }

    private function session(string $token): ?array {
        return self::SESSIONS[$token] ?? null;
    }
}
