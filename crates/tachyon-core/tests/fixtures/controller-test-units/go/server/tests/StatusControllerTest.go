// @Test(StatusTestSetup, {"partition":"tests"})
type StatusControllerTest struct {
	behaviors *BehaviorContext
}

func (test *StatusControllerTest) TestGetStatusSaysOk() {
	Assert.Equals("ok", (&StatusController{}).GetStatus())
}
