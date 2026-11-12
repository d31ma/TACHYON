package tac

type Passkeys struct{}

func (Passkeys) Supported() (bool, error) {
	value := false
	err := Web.Call("passkeys", "supported", nil, &value)
	return value, err
}

func (Passkeys) Check() error {
	return Web.Call("passkeys", "check", nil, nil)
}

func (Passkeys) Create(optionsJSON string) error {
	return Web.Call("passkeys", "create", []any{optionsJSON}, nil)
}

func (Passkeys) Get(optionsJSON string) error {
	return Web.Call("passkeys", "get", []any{optionsJSON}, nil)
}

func (Passkeys) Offer(optionsJSON string) error {
	return Web.Call("passkeys", "offer", []any{optionsJSON}, nil)
}

func (Passkeys) Cancel() error {
	return Web.Call("passkeys", "cancel", nil, nil)
}
