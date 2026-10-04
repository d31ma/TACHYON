-- @Test(PriceTestSetup, {"partition":"tests"})
---@class PriceUtilsTest
PriceUtilsTest = {}

function PriceUtilsTest:test_label_shows_dollars()
  Assert.equals("$12.50", PriceUtils.label(1250))
end

function PriceUtilsTest:test_basket_uses_the_utility()
  Assert.equals("$12.50", self:construct(BasketService):total_label())
end
