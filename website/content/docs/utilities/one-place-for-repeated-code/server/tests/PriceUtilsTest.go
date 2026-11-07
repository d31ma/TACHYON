// @Test(PriceTestSetup, {"partition":"tests"})
type PriceUtilsTest struct {
	behaviors *BehaviorContext
}

func (test *PriceUtilsTest) TestLabelShowsDollars() {
	Assert.Equals("$12.50", PriceUtils{}.Label(1250))
}
