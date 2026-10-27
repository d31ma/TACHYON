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
	resources   requestResources
	warm        bool
}

type requestResources struct {
	Endpoint string `json:"endpoint"`
	Token    string `json:"token"`
}

const defaultDeadlineMilliseconds = 30_000.0
const longestDeadlineSeconds = 86_400.0
const largestFrameBytes = 16 * 1024 * 1024
const protocolErrorExitCode = 70

var current = begin()

func begin() invocation {
	started := time.Now()
	environment := relayEnvironment()
	if os.Getenv("YON_HANDLER_WARM") == "1" {
		return invocation{requestID: "unknown", environment: environment, warm: true}
	}
	resources := requestResources{Endpoint: os.Getenv("YON_RESOURCE_ENDPOINT"), Token: os.Getenv("YON_RESOURCE_TOKEN")}
	return opened(started, readRequest(), resources, environment)
}

func opened(started time.Time, request []byte, resources requestResources, environment []string) invocation {
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
		resources:   resources,
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
	payload, failure := readFrame()
	return payload, failure == nil
}

func readFrame() ([]byte, error) {
	prefix := make([]byte, 4)
	if _, failure := io.ReadFull(os.Stdin, prefix); failure != nil {
		return nil, failure
	}
	length := int(prefix[0])<<24 | int(prefix[1])<<16 | int(prefix[2])<<8 | int(prefix[3])
	if length > largestFrameBytes {
		os.Exit(protocolErrorExitCode)
	}
	payload := make([]byte, length)
	if _, failure := io.ReadFull(os.Stdin, payload); failure != nil {
		return nil, io.ErrUnexpectedEOF
	}
	return payload, nil
}

func Warm() bool {
	return current.warm
}

func Next() bool {
	frame, failure := readFrame()
	if failure == io.EOF {
		return false
	}
	if failure != nil {
		os.Exit(protocolErrorExitCode)
	}
	started := time.Now()
	request, resources := withoutResources(frame)
	current = opened(started, request, resources, current.environment)
	current.warm = true
	return true
}

func withoutResources(frame []byte) ([]byte, requestResources) {
	var fields map[string]json.RawMessage
	if json.Unmarshal(frame, &fields) != nil || fields == nil {
		os.Exit(protocolErrorExitCode)
	}
	var operation string
	json.Unmarshal(fields["operation"], &operation)
	if operation == "socket.open" {
		os.Exit(protocolErrorExitCode)
	}
	found, hasResources := fields["resources"]
	if !hasResources {
		return frame, requestResources{}
	}
	delete(fields, "resources")
	var resources requestResources
	if json.Unmarshal(found, &resources) != nil {
		resources = requestResources{}
	}
	request, failure := json.Marshal(fields)
	if failure != nil {
		os.Exit(protocolErrorExitCode)
	}
	return request, resources
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
		if upper == "YON_APPLICATION_JSON" || upper == "YON_HANDLER_WARM" || strings.HasPrefix(upper, "YON_RESOURCE_") {
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

func ResourceEndpoint() string {
	return current.resources.Endpoint
}

func ResourceToken() string {
	return current.resources.Token
}
