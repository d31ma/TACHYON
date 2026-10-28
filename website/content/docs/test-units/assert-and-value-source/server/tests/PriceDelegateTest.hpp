#pragma once
#include <string>

class [[Test(PriceTestSetup)]] PriceDelegateTest {
public:
    void testAddsTheTax() {
        PriceDelegate prices = construct<PriceDelegate>();
        Assert::equals(1200, prices.withTax(1000));
    }

    [[ValueSource([0, 5, 1000])]]
    void testNeverLowersAPrice(int cents) {
        PriceDelegate prices = construct<PriceDelegate>();
        Assert::isTrue(prices.withTax(cents) >= cents, "the tax never lowers a price");
    }

    void testRefusesANegativePrice() {
        PriceDelegate prices = construct<PriceDelegate>();
        std::string error = Assert::throws([&prices] { prices.withTax(-1); });
        Assert::equals("A price is never negative.", error);
    }
};
