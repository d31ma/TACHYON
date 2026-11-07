package tac

type PageVisibility struct{}

func (PageVisibility) Visible() (bool, error) {
	value := false
	err := Web.Call("pageVisibility", "visible", nil, &value)
	return value, err
}
