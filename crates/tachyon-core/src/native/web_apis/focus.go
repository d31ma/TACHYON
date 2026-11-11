package tac

type Focus struct{}

func (Focus) Move(target string) error {
	return Web.Call("focus", "move", []any{target}, nil)
}

func (Focus) Trap(container string) error {
	return Web.Call("focus", "trap", []any{container}, nil)
}

func (Focus) Release() error {
	return Web.Call("focus", "release", nil, nil)
}
