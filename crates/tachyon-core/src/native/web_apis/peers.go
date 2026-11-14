package tac

type Peers struct{}

func (Peers) Supported() (bool, error) {
	value := false
	err := Web.Call("peers", "supported", nil, &value)
	return value, err
}

func (Peers) Open(username string, credential string) (string, error) {
	connection := ""
	err := Web.Call("peers", "open", []any{username, credential}, &connection)
	return connection, err
}

func (Peers) Offer(connection string) error {
	return Web.Call("peers", "offer", []any{connection}, nil)
}

func (Peers) Answer(connection string, offerSDP string) error {
	return Web.Call("peers", "answer", []any{connection, offerSDP}, nil)
}

func (Peers) Accept(connection string, answerSDP string) error {
	return Web.Call("peers", "accept", []any{connection, answerSDP}, nil)
}

func (Peers) AddCandidate(connection string, candidateJSON string) error {
	return Web.Call("peers", "addCandidate", []any{connection, candidateJSON}, nil)
}

func (Peers) Restart(connection string) error {
	return Web.Call("peers", "restart", []any{connection}, nil)
}

func (Peers) OpenChannel(connection string, label string, ordered bool) (string, error) {
	channel := ""
	err := Web.Call("peers", "openChannel", []any{connection, label, ordered}, &channel)
	return channel, err
}

func (Peers) Send(connection string, channel string, data string) (bool, error) {
	queued := false
	err := Web.Call("peers", "send", []any{connection, channel, data}, &queued)
	return queued, err
}

func (Peers) CloseChannel(connection string, channel string) error {
	return Web.Call("peers", "closeChannel", []any{connection, channel}, nil)
}

func (Peers) Close(connection string) error {
	return Web.Call("peers", "close", []any{connection}, nil)
}
