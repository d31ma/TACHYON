// @Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
type OrderServiceTest struct {
	behaviors *BehaviorContext
	// @Fake
	ordersClient *OrdersClient
}

func (test *OrderServiceTest) TestPutSendsThePathTheHeaderAndTheBody() {
	service := &OrderService{}
	test.construct(service)
	sent, err := service.Replace()
	Assert.Equals(nil, err)
	Assert.Equals("PUT", sent.Method)
	Assert.Equals("/api/orders/a-7", sent.Path)
	Assert.Equals("north", sent.Header)
	Assert.Equals("hammer", sent.Item)
}

func (test *OrderServiceTest) TestPatchSendsThePathTheHeaderAndTheBody() {
	service := &OrderService{}
	test.construct(service)
	sent, err := service.Change()
	Assert.Equals(nil, err)
	Assert.Equals("PATCH", sent.Method)
	Assert.Equals("/api/orders/a-7", sent.Path)
	Assert.Equals("south", sent.Header)
	Assert.Equals("nails", sent.Item)
}

func (test *OrderServiceTest) TestDeleteSendsThePathAndTheHeaderWithoutABody() {
	service := &OrderService{}
	test.construct(service)
	sent, err := service.Remove()
	Assert.Equals(nil, err)
	Assert.Equals("DELETE", sent.Method)
	Assert.Equals("/api/orders/a-7", sent.Path)
	Assert.Equals("duplicate", sent.Header)
	Assert.Equals("none", sent.Item)
}
