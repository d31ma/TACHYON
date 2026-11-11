package tac

type Push struct{}

func (Push) Supported() (bool, error) {
	value := false
	err := Web.Call("push", "supported", nil, &value)
	return value, err
}

func (Push) Subscribe(applicationServerKey string) error {
	return Web.Call("push", "subscribe", []any{applicationServerKey}, nil)
}

func (Push) Subscription() error {
	return Web.Call("push", "subscription", nil, nil)
}

func (Push) Unsubscribe() error {
	return Web.Call("push", "unsubscribe", nil, nil)
}
