#[Test(PriceTestSetup, {"partition":"tests"})]
class PriceUtilsTest
  def test_label_shows_dollars
    Assert.equals("$12.50", PriceUtils.label(1250))
  end

  def test_basket_uses_the_utility
    Assert.equals("$12.50", BasketService.new.total_label)
  end
end
