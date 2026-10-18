#pragma once
#include <string>

class [[Test(PriceTestSetup)]] PriceHelperTest {
public:
    void testAddsTheTax() {
        PriceHelper prices = construct<PriceHelper>();
        Assert::equals(1200, prices.withTax(1000));
    }

    [[ValueSource([0, 5, 1000])]]
    void testNeverLowersAPrice(int cents) {
        PriceHelper prices = construct<PriceHelper>();
        Assert::isTrue(prices.withTax(cents) >= cents, "the tax never lowers a price");
    }

    void testRefusesANegativePrice() {
        PriceHelper prices = construct<PriceHelper>();
        std::string error = Assert::throws([&prices] { prices.withTax(-1); });
        Assert::equals("A price is never negative.", error);
    }
};
