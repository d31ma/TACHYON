// @Test(GreetingTestSetup, {"partition":"tests"})
type GreetingControllerTest struct {
	behaviors *BehaviorContext
}

func (test *GreetingControllerTest) TestGetGreetingSaysHello() {
	controller := &GreetingController{}
	greeting := controller.GetGreeting()
	Assert.Equals("Hello from Yon", greeting.Message)
}
