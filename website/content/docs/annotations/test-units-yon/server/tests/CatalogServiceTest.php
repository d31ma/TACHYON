<?php
#[Test(CatalogTestSetup::class, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest {
    #[Fake]
    public CatalogCounterHelper $catalogCounterHelper;

    public function testDoublesTheCount(): void {
        $this->catalogCounterHelper = new class extends CatalogCounterHelper {
            public function count(): int {
                return 4;
            }
        };
        $service = new CatalogService($this->catalogCounterHelper);
        Assert::equals(8, $service->total());
    }

    public function testOpensAContext(): void {
        Assert::isTrue($this->behaviors !== null, 'a context');
    }
}
