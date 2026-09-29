// @Test(PriceTestSetup)
type PriceUtilsTest struct {
	behaviors *BehaviorContext
}

func (test *PriceUtilsTest) negativePrice() error {
	_, err := PriceUtils{}.WithTax(-1)
	return err
}

func (test *PriceUtilsTest) TestAddsTheTax() {
	total, err := PriceUtils{}.WithTax(1000)
	Assert.Equals(1200, total)
	Assert.Equals(nil, err)
}

// @ValueSource([0, 5, 1000])
func (test *PriceUtilsTest) TestNeverLowersAPrice(cents int64) {
	total, _ := PriceUtils{}.WithTax(cents)
	Assert.IsTrue(total >= cents, "the tax never lowers a price")
}

func (test *PriceUtilsTest) TestRefusesANegativePrice() {
	err := Assert.Throws(test.negativePrice)
	Assert.Equals("A price is never negative.", err.Error())
}
