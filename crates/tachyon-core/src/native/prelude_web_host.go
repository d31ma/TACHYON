package tac

import (
	"encoding/json"
	"errors"
)

type WebHost struct {
	ask   func(request string) string
	fakes map[string]any
}

var Web = &WebHost{}

func (host *WebHost) Connect(ask func(request string) string) {
	host.ask = ask
}

func (host *WebHost) Call(api string, call string, arguments []any, target any) error {
	request, _ := json.Marshal(map[string]any{"api": api, "call": call, "args": arguments})
	answer := host.parse(host.send(api+"."+call, string(request)))
	return answer.into(target)
}

func (host *WebHost) send(key string, request string) string {
	if host.ask == nil {
		return host.fakeAnswer(key)
	}
	return host.ask(request)
}

func (host *WebHost) fake(key string, value any) {
	if host.fakes == nil {
		host.fakes = map[string]any{}
	}
	host.fakes[key] = value
}

func (host *WebHost) fakeAnswer(key string) string {
	value, found := host.fakes[key]
	if !found {
		return `{"error":"This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes{}.Answer in the test unit."}`
	}
	answer, _ := json.Marshal(map[string]any{"value": value})
	return string(answer)
}

func (host *WebHost) parse(text string) webAnswer {
	answer := webAnswer{}
	_ = json.Unmarshal([]byte(text), &answer)
	return answer
}

func (answer webAnswer) into(target any) error {
	if answer.Error != "" {
		return errors.New(answer.Error)
	}
	return answer.decode(target)
}

func (answer webAnswer) decode(target any) error {
	if target == nil {
		return nil
	}
	return json.Unmarshal(answer.Value, target)
}
