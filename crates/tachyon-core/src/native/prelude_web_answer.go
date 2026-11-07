package tac

import "encoding/json"

type webAnswer struct {
	Value json.RawMessage `json:"value"`
	Error string          `json:"error"`
}
