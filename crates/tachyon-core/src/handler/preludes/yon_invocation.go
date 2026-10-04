package yoninvocation

import (
	"encoding/json"
	"io"
	"math"
	"os"
	"strings"
	"time"
)

type invocation struct {
	request     string
	deadline    time.Time
	budget      time.Duration
	requestID   string
	environment []string
}

const defaultDeadlineMilliseconds = 30_000.0
const longestDeadlineSeconds = 86_400.0
const largestFrameBytes = 16 * 1024 * 1024
const protocolErrorExitCode = 70

var current = begin()

func begin() invocation {
	started := time.Now()
	environment := relayEnvironment()
	request := readRequest()
	var fields map[string]any
	json.Unmarshal(request, &fields)
	seconds := deadlineMilliseconds(fields) / 1000
	seconds = math.Min(math.Max(seconds, 0), longestDeadlineSeconds)
	budget := time.Duration(seconds * float64(time.Second))
	return invocation{
		request:     string(request),
		deadline:    started.Add(budget),
		budget:      budget,
		requestID:   requestID(fields),
		environment: environment,
	}
}

func readRequest() []byte {
	if os.Getenv("YON_SOCKET_FRAMES") != "1" {
		request, _ := io.ReadAll(os.Stdin)
		return request
	}
	request, _ := ReadFrame()
	return request
}

func ReadFrame() ([]byte, bool) {
	prefix := make([]byte, 4)
	if _, failure := io.ReadFull(os.Stdin, prefix); failure != nil {
		return nil, false
	}
	length := int(prefix[0])<<24 | int(prefix[1])<<16 | int(prefix[2])<<8 | int(prefix[3])
	if length > largestFrameBytes {
		os.Exit(protocolErrorExitCode)
	}
	payload := make([]byte, length)
	if _, failure := io.ReadFull(os.Stdin, payload); failure != nil {
		return nil, false
	}
	return payload, true
}

func Renew() {
	current.deadline = time.Now().Add(current.budget)
}

func deadlineMilliseconds(fields map[string]any) float64 {
	milliseconds, isNumber := fields["deadline_ms"].(float64)
	if !isNumber {
		return defaultDeadlineMilliseconds
	}
	return milliseconds
}

func requestID(fields map[string]any) string {
	id, isText := fields["request_id"].(string)
	if !isText {
		return "unknown"
	}
	return id
}

func relayEnvironment() []string {
	kept := []string{}
	for _, entry := range os.Environ() {
		name, _, _ := strings.Cut(entry, "=")
		upper := strings.ToUpper(name)
		if upper == "YON_APPLICATION_JSON" || strings.HasPrefix(upper, "YON_RESOURCE_") {
			continue
		}
		kept = append(kept, entry)
	}
	return kept
}

func Request() string {
	return current.request
}

func Deadline() time.Time {
	return current.deadline
}

func TimeLeft() time.Duration {
	return time.Until(current.deadline)
}

func RequestID() string {
	return current.requestID
}

func RelayEnvironment() []string {
	return append([]string{}, current.environment...)
}
