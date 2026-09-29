-- @Test(PriceTestSetup)
---@class PriceUtilsTest
PriceUtilsTest = {}

function PriceUtilsTest:test_adds_the_tax()
  Assert.equals(1200, PriceUtils.with_tax(1000))
end

-- @ValueSource([0, 5, 1000])
---@param cents integer
function PriceUtilsTest:test_never_lowers_a_price(cents)
  Assert.is_true(PriceUtils.with_tax(cents) >= cents, "the tax never lowers a price")
end

function PriceUtilsTest:test_refuses_a_negative_price()
  local failure = Assert.throws(function()
    PriceUtils.with_tax(-1)
  end)
  Assert.equals("A price is never negative.", failure)
end
