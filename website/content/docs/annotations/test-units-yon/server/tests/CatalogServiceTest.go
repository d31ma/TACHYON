// @Test(CatalogTestSetup, {"partition":"tests"})
type CatalogServiceTest struct {
	behaviors *BehaviorContext
}

func (test *CatalogServiceTest) TestOpensAContext() {
	Assert.IsTrue(test.behaviors != nil, "a context")
}
