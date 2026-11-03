#[Test(PriceTestSetup)]
class PriceUtilsTest
  def test_adds_the_tax
    Assert.equals(1200, PriceUtils.with_tax(1000))
  end

  #[ValueSource([0, 5, 1000])]
  def test_never_lowers_a_price(cents)
    Assert.is_true(PriceUtils.with_tax(cents) >= cents, "the tax never lowers a price")
  end

  def test_refuses_a_negative_price
    error = Assert.throws { PriceUtils.with_tax(-1) }
    Assert.equals("A price is never negative.", error.message)
  end
end
