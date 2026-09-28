// @Test(StatusTestSetup, {"partition":"tests"})
type StatusControllerTest struct {
	behaviors *BehaviorContext
}

func (test *StatusControllerTest) TestGetStatusSaysOk() error {
	if (&StatusController{}).GetStatus() != "ok" {
		return &BehaviorError{Code: "adapter", Message: "expected ok"}
	}
	return nil
}
