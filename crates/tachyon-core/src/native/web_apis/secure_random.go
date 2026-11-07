package tac

type SecureRandom struct{}

func (SecureRandom) Bytes(count int) ([]byte, error) {
	values := []int{}
	err := Web.Call("secureRandom", "bytes", []any{count}, &values)
	return bytesOf(values), err
}

func (SecureRandom) UUID() (string, error) {
	value := ""
	err := Web.Call("secureRandom", "uuid", nil, &value)
	return value, err
}

func bytesOf(values []int) []byte {
	bytes := make([]byte, 0, len(values))
	for _, value := range values {
		bytes = append(bytes, byte(value))
	}
	return bytes
}
