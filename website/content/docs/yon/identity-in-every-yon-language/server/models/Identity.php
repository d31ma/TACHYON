<?php
class Identity {
    public function __construct(
        public string $user,
        /** @var list<string> */
        public array $roles,
    ) {}
}
