package tac

type Connectivity struct{}

func (Connectivity) Online() (bool, error) {
	value := false
	err := Web.Call("connectivity", "online", nil, &value)
	return value, err
}
