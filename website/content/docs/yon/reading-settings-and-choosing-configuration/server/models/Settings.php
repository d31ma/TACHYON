<?php
class Settings {
    public function __construct(
        public string $profile,
        public int $pageSize,
    ) {}
}
