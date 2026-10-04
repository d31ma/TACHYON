package tac

import (
	"runtime"
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
	owner  string
	answer chan string
}

var currentTask *task
var lastTask int
var lastTicket int
var httpTickets = map[int]*httpTicket{}
var TaskDone func(id int, answer string)
var HTTPStart func(ticket int, request string)
var HTTPCancel func(ticket int)

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
	if currentTask == nil {
		return httpOutsideTask
	}
	lastTicket++
	ticket := &httpTicket{owner: executingOwner(), answer: make(chan string, 1)}
	httpTickets[lastTicket] = ticket
	HTTPStart(lastTicket, request)
	paused := suspend()
	answer := <-ticket.answer
	paused.resume()
	return answer
}

func SettleHTTP(id int, answer string) {
	ticket := httpTickets[id]
	delete(httpTickets, id)
	if ticket != nil {
		ticket.answer <- answer
	}
}

func cancelHTTP(owner string) {
	for id, ticket := range httpTickets {
		if ticket.owner != owner {
			continue
		}
		delete(httpTickets, id)
		if HTTPCancel != nil {
			HTTPCancel(id)
		}
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
	handling = false
	currentTask = nil
	select {
	case paused.task.waiting <- struct{}{}:
	default:
	}
	return paused
}

func (paused pause) resume() {
	for handling {
		runtime.Gosched()
	}
	handling = true
	executing = paused.owner
	currentTask = paused.task
}
