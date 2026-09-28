// @Test(CatalogTestSetup, {"partition":"tests"})
type CatalogServiceTest struct {
	behaviors *BehaviorContext
}

func (test *CatalogServiceTest) TestOpensAContext() error {
	if test.behaviors == nil {
		return &BehaviorError{Code: "adapter", Message: "no context"}
	}
	return nil
}
