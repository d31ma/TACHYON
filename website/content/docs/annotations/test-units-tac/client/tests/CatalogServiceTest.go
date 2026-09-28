import "errors"

// @Test(CatalogTestSetup, {"partition":"tests"})
type CatalogServiceTest struct {
	behaviors *BehaviorContext
}

func (test *CatalogServiceTest) TestHasAContext() error {
	if test.behaviors == nil {
		return errors.New("no context")
	}
	return nil
}
