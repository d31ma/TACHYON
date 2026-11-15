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

func (Passkeys) Create(optionsJSON string, prfKey string) error {
	return Web.Call("passkeys", "create", []any{optionsJSON, prfKey}, nil)
}

func (Passkeys) Get(optionsJSON string, prfKey string) error {
	return Web.Call("passkeys", "get", []any{optionsJSON, prfKey}, nil)
}

func (Passkeys) Offer(optionsJSON string, prfKey string) error {
	return Web.Call("passkeys", "offer", []any{optionsJSON, prfKey}, nil)
}

func (Passkeys) Cancel() error {
	return Web.Call("passkeys", "cancel", nil, nil)
}
