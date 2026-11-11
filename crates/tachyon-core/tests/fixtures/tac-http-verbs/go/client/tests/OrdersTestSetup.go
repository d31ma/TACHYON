// @TestSetup
type OrdersTestSetup struct{}

func (OrdersTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["orders"] = OrdersTestSetup{}
	return config, nil
}

func (OrdersTestSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	if request.Path == "/api/claims/a-7" {
		return nil, &BehaviorError{Code: "HTTP_STATUS", Message: "private fake text", Status: 409, Body: map[string]any{"reason": "taken", "holder": "south"}}
	}
	header := ""
	for _, value := range request.Headers {
		header, _ = value.(string)
	}
	item := "none"
	if request.HasBody {
		body, _ := request.Body.(map[string]any)
		item, _ = body["item"].(string)
	}
	return map[string]any{"method": request.Method, "path": request.Path, "header": header, "item": item}, nil
}
