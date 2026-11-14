package tac

import "encoding/base64"

func (message PeerMessage) Bytes() ([]byte, error) {
	return base64.RawURLEncoding.DecodeString(message.Data)
}
