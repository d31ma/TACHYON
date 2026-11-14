package tac

type CodeScanner struct{}

func (CodeScanner) Supported() (bool, error) {
	value := false
	err := Web.Call("codeScanner", "supported", nil, &value)
	return value, err
}

func (CodeScanner) Scan() error {
	return Web.Call("codeScanner", "scan", nil, nil)
}

func (CodeScanner) Cancel() error {
	return Web.Call("codeScanner", "cancel", nil, nil)
}
