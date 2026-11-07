#pragma once
#include "../services/TallyService.hpp"

class [[Test(TallyTestSetup)]] TallyServiceTest {
public:
    void testAddsAmounts() {
        TallyService tally;
        tally.add(2);
        tally.add(3);
        Assert::equals(5, tally.count, "the count");
    }

    void testDoublesAnOrder() {
        TallyService tally;
        PlacedOrder order{"paid", 4, {}};
        Assert::equals(8, tally.doubled(order).total);
    }

    void testReadsJson() {
        PlacedOrder order = tachyon::json::JsonSerializer::deserialize<PlacedOrder>(R"({"status":"open","total":2})");
        Assert::equals("open", order.status);
        Assert::throws([] { tachyon::json::JsonNode::parse("{nope"); });
    }
};
