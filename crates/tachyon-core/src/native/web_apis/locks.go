package tac

type Locks struct{}

func (Locks) Request(name string) error {
	return Web.Call("locks", "request", []any{name}, nil)
}

func (Locks) Release(name string) error {
	return Web.Call("locks", "release", []any{name}, nil)
}
