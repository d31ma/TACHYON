package tac

import "encoding/base64"

func (chunk FileChunkRead) Bytes() ([]byte, error) {
	return base64.StdEncoding.DecodeString(chunk.Data)
}
