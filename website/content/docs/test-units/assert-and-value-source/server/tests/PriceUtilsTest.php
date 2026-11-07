<?php
#[Test(PriceTestSetup::class)]
class PriceUtilsTest {
    public function testAddsTheTax(): void {
        Assert::equals(1200, PriceUtils::withTax(1000));
    }

    #[ValueSource([0, 5, 1000])]
    public function testNeverLowersAPrice(int $cents): void {
        assert(PriceUtils::withTax($cents) >= $cents, 'the tax never lowers a price');
    }

    public function testRefusesANegativePrice(): void {
        $error = Assert::throws(fn() => PriceUtils::withTax(-1));
        Assert::equals('A price is never negative.', $error->getMessage());
    }
}
