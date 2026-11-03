package tac

type Notifications struct{}

func (Notifications) Permission() (string, error) {
	value := ""
	err := Web.Call("notifications", "permission", nil, &value)
	return value, err
}

func (Notifications) RequestPermission() error {
	return Web.Call("notifications", "requestPermission", nil, nil)
}

func (Notifications) Show(title string, body string) error {
	return Web.Call("notifications", "show", []any{title, body}, nil)
}
