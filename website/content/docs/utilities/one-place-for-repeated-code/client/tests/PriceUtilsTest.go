// @Test(PriceTestSetup, {"partition":"tests"})
type PriceUtilsTest struct {
	behaviors *BehaviorContext
}

func (test *PriceUtilsTest) TestLabelShowsDollars() {
	Assert.Equals("$12.50", PriceUtils{}.Label(1250))
}

func (test *PriceUtilsTest) TestBasketUsesTheUtility() {
	basket := &BasketService{}
	Assert.Equals(nil, basket.Start())
	Assert.Equals("$12.50", basket.TotalLabel())
}
