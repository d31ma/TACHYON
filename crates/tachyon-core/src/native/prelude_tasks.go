package tac

import (
	"encoding/json"
	"strconv"
)

type task struct {
	id       int
	done     chan string
	waiting  chan struct{}
	detached bool
}

type pause struct {
	owner *ownerActivation
	task  *task
}

type httpTicket struct {
	owner     string
	answer    chan string
	cancelled bool
}

var currentTask *task
var lastTask int
var lastTicket int
var httpTickets = map[int]*httpTicket{}
var turns []chan struct{}
var TaskDone func(id int, answer string)
var HTTPStart func(ticket int, request string)
var HTTPCancel func(ticket int) bool
var SocketStart func(ticket int, owner string, request string)

const httpMissing = `{"error":{"code":"HTTP_MISSING"}}`
const httpCancelled = `{"error":{"code":"CANCELLED"}}`
const httpOutsideTask = `{"error":{"code":"HTTP_REQUEST"}}`

func InvokeTask(text string) string {
	if !waits(text) {
		return Invoke(text)
	}
	lastTask++
	started := &task{id: lastTask, done: make(chan string, 1), waiting: make(chan struct{}, 1)}
	go started.run(text)
	select {
	case answer := <-started.done:
		return answer
	case <-started.waiting:
		started.detached = true
		return `{"task":` + strconv.Itoa(started.id) + `}`
	}
}

func waits(text string) bool {
	request, valid := readRequest(text)
	return valid && (request.operation == "call" || request.operation == "event")
}

func (started *task) run(text string) {
	currentTask = started
	answer := Invoke(text)
	currentTask = nil
	if started.detached && TaskDone != nil {
		TaskDone(started.id, answer)
		return
	}
	started.done <- answer
}

func Exchange(request string) string {
	if HTTPStart == nil {
		return httpMissing
	}
	return awaitTicket(func(id int, owner string) {
		HTTPStart(id, request)
	})
}

type SocketError struct {
	Code    string
	Message string
}

func (failure *SocketError) Error() string {
	return failure.Code + ": " + failure.Message
}

func (failure *SocketError) BehaviorCode() string {
	return failure.Code
}

func SendSocket(client string, message any) error {
	text, err := json.Marshal(message)
	if err != nil || len(text) > 65536 {
		return &SocketError{Code: "HTTP_BODY", Message: "A socket message is JSON of at most 64 KiB."}
	}
	request, _ := json.Marshal(map[string]any{"client": client, "message": json.RawMessage(text)})
	answer := map[string]any{}
	_ = json.Unmarshal([]byte(socketSend(string(request))), &answer)
	failure, failed := answer["error"].(map[string]any)
	if _, sent := answer["value"]; failed || !sent {
		return &SocketError{Code: socketCode(failure["code"]), Message: "The socket message was not sent."}
	}
	return nil
}

func socketSend(request string) string {
	if SocketStart == nil {
		return httpMissing
	}
	return awaitTicket(func(id int, owner string) {
		SocketStart(id, owner, request)
	})
}

func socketCode(code any) string {
	for _, known := range []string{"HTTP_MISSING", "HTTP_REQUEST", "CANCELLED"} {
		if code == known {
			return known
		}
	}
	return "HTTP_NETWORK"
}

func awaitTicket(start func(id int, owner string)) string {
	if currentTask == nil {
		return httpOutsideTask
	}
	if executing != nil && executing.disposed {
		return httpCancelled
	}
	lastTicket++
	ticket := &httpTicket{owner: executingOwner(), answer: make(chan string, 1)}
	httpTickets[lastTicket] = ticket
	start(lastTicket, ticket.owner)
	paused := suspend()
	answer := <-ticket.answer
	paused.resume()
	return answer
}

func SettleHTTP(id int, answer string) {
	ticket := httpTickets[id]
	delete(httpTickets, id)
	if ticket == nil {
		return
	}
	if ticket.cancelled {
		answer = httpCancelled
	}
	ticket.answer <- answer
}

func cancelHTTP(owner string) {
	for id, ticket := range httpTickets {
		if ticket.owner != owner || ticket.cancelled {
			continue
		}
		ticket.cancelled = true
		if HTTPCancel != nil && HTTPCancel(id) {
			continue
		}
		delete(httpTickets, id)
		ticket.answer <- httpCancelled
	}
}

func executingOwner() string {
	if executing == nil {
		return ""
	}
	return executing.owner
}

func suspend() pause {
	paused := pause{owner: executing, task: currentTask}
	executing = nil
	currentTask = nil
	yieldTurn()
	select {
	case paused.task.waiting <- struct{}{}:
	default:
	}
	return paused
}

func (paused pause) resume() {
	if handling {
		turn := make(chan struct{})
		turns = append(turns, turn)
		<-turn
	}
	handling = true
	executing = paused.owner
	currentTask = paused.task
}

func yieldTurn() {
	handling = false
	if len(turns) == 0 {
		return
	}
	next := turns[0]
	turns = turns[1:]
	handling = true
	close(next)
}
