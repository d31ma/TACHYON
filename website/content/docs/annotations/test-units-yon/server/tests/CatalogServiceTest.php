<?php
#[Test(CatalogTestSetup::class, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest {
    public function testOpensAContext(): void {
        Assert::isTrue($this->behaviors !== null, 'a context');
    }
    private function twice(int $value): int { return $value * 2; }
    public function testDoublesAValue(): void {
        Assert::equals(8, $this->twice(4));
    }
}
