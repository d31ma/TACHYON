<?php
#[Test(CatalogTestSetup::class, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest {
    #[Fake]
    public CatalogCounterDelegate $catalogCounterDelegate;

    public function testDoublesTheCount(): void {
        $this->catalogCounterDelegate = new class extends CatalogCounterDelegate {
            public function count(): int {
                return 4;
            }
        };
        $service = new CatalogService($this->catalogCounterDelegate);
        Assert::equals(8, $service->total());
    }

    public function testOpensAContext(): void {
        Assert::isTrue($this->behaviors !== null, 'a context');
    }
}
