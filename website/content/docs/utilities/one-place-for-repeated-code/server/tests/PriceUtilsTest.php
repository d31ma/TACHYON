<?php
#[Test(PriceTestSetup::class, {"partition":"tests"})]
class PriceUtilsTest {
    public function testLabelShowsDollars(): void {
        Assert::equals('$12.50', PriceUtils::label(1250));
    }
}
