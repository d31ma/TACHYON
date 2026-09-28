<?php
#[Test(CatalogTestSetup::class, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest {
    public function testOpensAContext(): void {
        if ($this->behaviors === null) throw new Exception('no context');
    }
    private function twice(int $value): int { return $value * 2; }
    public function testDoublesAValue(): void {
        if ($this->twice(4) !== 8) throw new Exception('expected 8');
    }
}
