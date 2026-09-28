package main

import (
	"encoding/json"
	"os"
	"reflect"
	"sort"
	"strings"
	"sync"
)

type ApplicationEventPublisher struct{}

func (publisher *ApplicationEventPublisher) PublishEvent(value any, retain bool) {
	kind := reflect.TypeOf(value)
	if kind != nil && kind.Kind() == reflect.Pointer {
		kind = kind.Elem()
	}
	if kind == nil || kind.Kind() != reflect.Struct || kind.Name() == "" {
		os.Stderr.WriteString("Yon local event publication failed.\n")
		return
	}
	__yonEventPublish(kind.Name(), value, retain)
}

func __yonEventRead[T any](value any) T {
	var model T
	encoded, err := json.Marshal(value)
	if err != nil || json.Unmarshal(encoded, &model) != nil {
		panic("Local event payload cannot be read")
	}
	return model
}

func __yonEventFits(models map[string]any, shape string, value any) bool {
	if inner, optional := strings.CutPrefix(shape, "?"); optional {
		return value == nil || __yonEventFits(models, inner, value)
	}
	if item, isList := strings.CutPrefix(shape, "[]"); isList {
		items, isArray := value.([]any)
		if !isArray {
			return false
		}
		for _, entry := range items {
			if !__yonEventFits(models, item, entry) {
				return false
			}
		}
		return true
	}
	switch shape {
	case "any":
		return true
	case "string":
		_, isText := value.(string)
		return isText
	case "integer":
		number, isNumber := value.(float64)
		return isNumber && number == float64(int64(number))
	case "number":
		_, isNumber := value.(float64)
		return isNumber
	case "boolean":
		_, isFlag := value.(bool)
		return isFlag
	}
	model, isModel := models[shape].(map[string]any)
	object, isObject := value.(map[string]any)
	if !isModel || !isObject {
		return false
	}
	fields, _ := model["fields"].(map[string]any)
	if open, _ := model["open"].(bool); !open {
		for key := range object {
			if _, declared := fields[key]; !declared {
				return false
			}
		}
	}
	for name, declared := range fields {
		field := __yonEventText(declared)
		inner, required := strings.CutPrefix(field, "!")
		found, present := object[name]
		if !present {
			if required {
				return false
			}
			continue
		}
		if !__yonEventFits(models, inner, found) {
			return false
		}
	}
	return true
}

type __yonEventMessage struct {
	name    string
	encoded string
}

type __yonEventState struct {
	queue    []__yonEventMessage
	retained map[string]string
	closed   bool
	topics   map[string]bool
	outbound []__yonEventMessage
}

var __yonEventLock sync.Mutex

var __yonEventCurrent *__yonEventState

func __yonEventPublish(name string, value any, retain bool) {
	encoded, err := json.Marshal(value)
	delivered := err == nil && len(encoded) <= 65536 && __yonEventEnqueue(name, string(encoded), retain)
	if !delivered {
		os.Stderr.WriteString("Yon local event publication failed.\n")
	}
}

func __yonEventEnqueue(name string, encoded string, retain bool) bool {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	state := __yonEventCurrent
	if state == nil || state.closed || len(state.queue) >= 128 {
		return false
	}
	if retain {
		state.retained[name] = encoded
	}
	state.queue = append(state.queue, __yonEventMessage{name: name, encoded: encoded})
	if state.topics[name] {
		state.outbound = append(state.outbound, __yonEventMessage{name: name, encoded: encoded})
	}
	return true
}

func __yonEventReceive(name string, encoded string) {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	if __yonEventCurrent != nil && !__yonEventCurrent.closed {
		__yonEventCurrent.queue = append(__yonEventCurrent.queue, __yonEventMessage{name: name, encoded: encoded})
	}
}

func __yonEventTakeOutbound() []__yonEventMessage {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	if __yonEventCurrent == nil {
		return nil
	}
	outbound := __yonEventCurrent.outbound
	__yonEventCurrent.outbound = nil
	return outbound
}

func __yonEventSendTopics() {
	for _, message := range __yonEventTakeOutbound() {
		command := map[string]any{"operation": "topic", "topic": message.name, "payload": json.RawMessage(message.encoded)}
		if _, err := yonResourceCall(command); err != nil {
			os.Stderr.WriteString("Yon topic publication failed.\n")
		}
	}
}

func __yonEventNext() (__yonEventMessage, bool) {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	state := __yonEventCurrent
	if state == nil || len(state.queue) == 0 {
		return __yonEventMessage{}, false
	}
	message := state.queue[0]
	state.queue = state.queue[1:]
	return message, true
}

func __yonEventReplay(name string) {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	state := __yonEventCurrent
	if state == nil {
		return
	}
	if encoded, present := state.retained[name]; present {
		state.queue = append(state.queue, __yonEventMessage{name: name, encoded: encoded})
	}
}

func __yonEventClearQueue() {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	if __yonEventCurrent != nil {
		__yonEventCurrent.queue = nil
	}
}

func __yonEventField(node any, key string) any {
	object, isObject := node.(map[string]any)
	if !isObject {
		return nil
	}
	return object[key]
}

func __yonEventText(node any) string {
	text, _ := node.(string)
	return text
}

func __yonEventOperand(node any, value any, event string) any {
	switch __yonEventText(__yonEventField(node, "k")) {
	case "lit":
		return __yonEventField(node, "v")
	case "event":
		return event
	}
	current := value
	segments, _ := __yonEventField(node, "path").([]any)
	for _, segment := range segments {
		switch typed := current.(type) {
		case map[string]any:
			current = typed[__yonEventText(segment)]
		case []any:
			index, isNumber := segment.(float64)
			if !isNumber || index < 0 || index != float64(int(index)) || int(index) >= len(typed) {
				return nil
			}
			current = typed[int(index)]
		default:
			return nil
		}
	}
	return current
}

func __yonEventTruthy(value any) bool {
	switch typed := value.(type) {
	case nil:
		return false
	case bool:
		return typed
	case string:
		return typed != ""
	case float64:
		return typed != 0
	}
	return true
}

func __yonEventCompare(operator string, left any, right any) bool {
	leftNumber, leftIsNumber := left.(float64)
	rightNumber, rightIsNumber := right.(float64)
	numbers := leftIsNumber && rightIsNumber
	leftText, leftIsText := left.(string)
	rightText, rightIsText := right.(string)
	texts := leftIsText && rightIsText
	if operator == "eq" || operator == "ne" {
		equal := false
		switch {
		case numbers:
			equal = leftNumber == rightNumber
		case texts:
			equal = leftText == rightText
		case left == nil && right == nil:
			equal = true
		default:
			leftFlag, leftIsFlag := left.(bool)
			rightFlag, rightIsFlag := right.(bool)
			equal = leftIsFlag && rightIsFlag && leftFlag == rightFlag
		}
		return (operator == "eq") == equal
	}
	order := 0
	switch {
	case numbers && leftNumber < rightNumber, texts && leftText < rightText:
		order = -1
	case numbers && leftNumber > rightNumber, texts && leftText > rightText:
		order = 1
	case numbers, texts:
		order = 0
	default:
		return false
	}
	switch operator {
	case "lt":
		return order < 0
	case "le":
		return order <= 0
	case "gt":
		return order > 0
	}
	return order >= 0
}

func __yonEventMatches(node any, value any, event string) bool {
	switch __yonEventText(__yonEventField(node, "k")) {
	case "and":
		return __yonEventMatches(__yonEventField(node, "l"), value, event) && __yonEventMatches(__yonEventField(node, "r"), value, event)
	case "or":
		return __yonEventMatches(__yonEventField(node, "l"), value, event) || __yonEventMatches(__yonEventField(node, "r"), value, event)
	case "not":
		return !__yonEventMatches(__yonEventField(node, "e"), value, event)
	case "truthy":
		return __yonEventTruthy(__yonEventOperand(__yonEventField(node, "e"), value, event))
	case "cmp":
		left := __yonEventOperand(__yonEventField(node, "l"), value, event)
		right := __yonEventOperand(__yonEventField(node, "r"), value, event)
		return __yonEventCompare(__yonEventText(__yonEventField(node, "op")), left, right)
	}
	return false
}

type __yonEventListener struct {
	name      string
	order     float64
	condition any
	models    map[string]any
	returns   string
	sequence  int
	callback  func(any) any
}

type __yonEvents struct {
	listeners  []__yonEventListener
	destroyers []func()
	deliveries int
	only       map[string]bool
}

func __yonNewEvents() *__yonEvents {
	__yonEventLock.Lock()
	defer __yonEventLock.Unlock()
	if __yonEventCurrent != nil {
		panic("Nested local event invocation")
	}
	__yonEventCurrent = &__yonEventState{retained: map[string]string{}, topics: map[string]bool{}}
	return &__yonEvents{}
}

func __yonNewTopicEvents(topics []string, only []string) *__yonEvents {
	events := __yonNewEvents()
	__yonEventLock.Lock()
	for _, topic := range topics {
		__yonEventCurrent.topics[topic] = true
	}
	__yonEventLock.Unlock()
	if only != nil {
		events.only = map[string]bool{}
		for _, key := range only {
			events.only[key] = true
		}
	}
	return events
}

func (events *__yonEvents) listen(options string, listener func(any) any, replay bool, key string) {
	if events.only != nil && !events.only[key] {
		return
	}
	var parsed map[string]any
	if json.Unmarshal([]byte(options), &parsed) != nil {
		panic("Compiled listener options")
	}
	order, _ := parsed["order"].(float64)
	models, _ := parsed["models"].(map[string]any)
	returns, _ := parsed["returns"].(string)
	names, _ := parsed["events"].([]any)
	for _, entry := range names {
		if len(events.listeners) >= 1024 {
			panic("Local listener capacity exceeded")
		}
		name := __yonEventText(entry)
		if replay {
			__yonEventReplay(name)
		}
		events.listeners = append(events.listeners, __yonEventListener{
			name:      name,
			order:     order,
			condition: parsed["condition"],
			models:    models,
			returns:   returns,
			sequence:  len(events.listeners),
			callback:  listener,
		})
	}
	sort.SliceStable(events.listeners, func(left int, right int) bool {
		if events.listeners[left].order != events.listeners[right].order {
			return events.listeners[left].order < events.listeners[right].order
		}
		return events.listeners[left].sequence < events.listeners[right].sequence
	})
}

func __yonEventRepublish(result any, returns string) {
	if returns == "" || result == nil {
		return
	}
	items := []any{result}
	list := reflect.ValueOf(result)
	if list.Kind() == reflect.Slice {
		items = nil
		for index := 0; index < list.Len() && index < 32; index++ {
			items = append(items, list.Index(index).Interface())
		}
	}
	for _, item := range items {
		if item != nil {
			__yonEventPublish(returns, item, false)
		}
	}
}

func __yonEventCall(callback func(any) any, value any) (result any, failed bool) {
	defer func() {
		if recover() != nil {
			failed = true
		}
	}()
	return callback(value), false
}

func (events *__yonEvents) drain() {
	for {
		message, present := __yonEventNext()
		if !present {
			__yonEventSendTopics()
			return
		}
		events.deliveries++
		if events.deliveries > 4096 {
			__yonEventClearQueue()
			os.Stderr.WriteString("Yon local event delivery bound exceeded.\n")
			return
		}
		for _, listener := range events.listeners {
			if listener.name != message.name {
				continue
			}
			var value any
			json.Unmarshal([]byte(message.encoded), &value)
			if listener.models != nil && !__yonEventFits(listener.models, message.name, value) {
				os.Stderr.WriteString("Yon local event payload does not fit the listener model.\n")
				continue
			}
			if listener.condition != nil && !__yonEventMatches(listener.condition, value, message.name) {
				continue
			}
			result, failed := __yonEventCall(listener.callback, value)
			if failed {
				__yonListenerFailed()
				continue
			}
			__yonEventRepublish(result, listener.returns)
		}
	}
}

func (events *__yonEvents) close() {
	__yonEventLock.Lock()
	if __yonEventCurrent != nil {
		__yonEventCurrent.closed = true
		__yonEventCurrent.queue = nil
	}
	__yonEventLock.Unlock()
	events.listeners = nil
	for index := len(events.destroyers) - 1; index >= 0; index-- {
		__yonEventDestroy(events.destroyers[index])
	}
	__yonEventLock.Lock()
	__yonEventCurrent = nil
	__yonEventLock.Unlock()
}

func __yonEventDestroy(destroy func()) {
	defer func() {
		if recover() != nil {
			__yonCleanupFailed()
		}
	}()
	destroy()
}

func __yonListenerFailed() {
	os.Stderr.WriteString("Yon local event listener failed.\n")
}

func __yonCleanupFailed() {
	os.Stderr.WriteString("Yon local lifecycle cleanup failed.\n")
}
