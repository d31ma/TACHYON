package tac

type Clock struct{}

func (Clock) NowMs() (float64, error) {
	value := 0.0
	err := Web.Call("clock", "now", nil, &value)
	return value, err
}

func (Clock) MonotonicMs() (float64, error) {
	value := 0.0
	err := Web.Call("clock", "monotonic", nil, &value)
	return value, err
}

func (Clock) TimeZone() (string, error) {
	value := ""
	err := Web.Call("clock", "timeZone", nil, &value)
	return value, err
}
