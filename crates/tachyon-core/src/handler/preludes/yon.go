package main

import (
	"bytes"
	yoncontext "context"
	"encoding/json"
	"errors"
	"encoding/base64"
	"io"
	yoniter "iter"
	"net"
	"net/http"
	yonurl "net/url"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"
	"yonhandler/yoninvocation"
)

type yonRequest struct {
	raw    string
	fields yonRequestFields
}

type yonRequestFields struct {
	Operation  string              `json:"operation"`
	Route      string              `json:"route"`
	Target     string              `json:"target"`
	Method     string              `json:"method"`
	Headers    map[string][]string `json:"headers"`
	Parameters map[string]string   `json:"parameters"`
	Body       *yonRequestBody     `json:"body"`
	Arguments  []json.RawMessage   `json:"arguments"`
}

type yonRequestBody struct {
	Encoding string `json:"encoding"`
	Data     string `json:"data"`
}

func (request *yonRequest) Method() string {
	return request.fields.Method
}

func (request *yonRequest) Route() string {
	return request.fields.Route
}

func (request *yonRequest) Parameter(name string) string {
	return request.fields.Parameters[name]
}

func (request *yonRequest) Header(name string) string {
	for key, values := range request.fields.Headers {
		if strings.EqualFold(key, name) && len(values) > 0 {
			return values[0]
		}
	}
	return ""
}

func (request *yonRequest) Body() string {
	if request.fields.Body == nil {
		return ""
	}
	return request.fields.Body.Data
}

func (request *yonRequest) JSON(target any) error {
	return json.Unmarshal([]byte(request.Body()), target)
}

func (request *yonRequest) Raw() string {
	return request.raw
}

type yonResponse struct {
	status  int
	headers map[string][]string
	body    string
	stream  *yonEventStream
}

func yonJSON(body string) *yonResponse {
	response := yonEmpty(200).Header("content-type", "application/json")
	response.body = body
	return response
}

func yonEmpty(status int) *yonResponse {
	return &yonResponse{status: status, headers: map[string][]string{}}
}

func (response *yonResponse) Status(status int) *yonResponse {
	response.status = status
	return response
}

func (response *yonResponse) Header(name string, value string) *yonResponse {
	response.headers[name] = append(response.headers[name], value)
	return response
}

type yonResponseFields struct {
	Status  int                 `json:"status"`
	Headers map[string][]string `json:"headers"`
	Body    string              `json:"body"`
}

func yonArgument[T any](request *yonRequest, index int) T {
	var value T
	if index < len(request.fields.Arguments) {
		_ = json.Unmarshal(request.fields.Arguments[index], &value)
	}
	return value
}

func yonNoRoute() error {
	return errors.New("No controller method answers this route.")
}

func yonAnswer(value any, failures ...error) *yonResponse {
	for _, failure := range failures {
		if failure == nil {
			continue
		}
		var resourceError YonResourceError
		if errors.As(failure, &resourceError) {
			return yonJSON(`{"error":"Yon managed resource operation failed."}`).Status(500)
		}
		panic(failure)
	}
	stream, isStream := value.(yonEventStream)
	if isStream {
		return &yonResponse{stream: &stream}
	}
	native, isNative := value.(*http.Response)
	if isNative {
		return yonNativeAnswer(native)
	}
	response, isResponse := value.(*yonResponse)
	if isResponse && response == nil {
		return yonEmpty(204)
	}
	if isResponse {
		return response
	}
	body, err := json.Marshal(value)
	if err != nil {
		return yonJSON(`{"error":"Handler returned an invalid JSON value."}`).Status(500)
	}
	return yonJSON(string(body))
}

func yonNativeRequest(request *yonRequest) *http.Request {
	host := request.Header("host")
	if host == "" {
		host = "localhost"
	}
	target := request.fields.Target
	if target == "" {
		target = request.fields.Route
	}
	body := []byte(request.Body())
	if request.fields.Body != nil && request.fields.Body.Encoding == "base64" {
		body, _ = base64.StdEncoding.DecodeString(request.fields.Body.Data)
	}
	native, err := http.NewRequest(request.fields.Method, "http://"+host+target, bytes.NewReader(body))
	if err != nil {
		native, _ = http.NewRequest(request.fields.Method, "http://localhost/", bytes.NewReader(body))
	}
	for name, values := range request.fields.Headers {
		for _, value := range values {
			native.Header.Add(name, value)
		}
	}
	native.Host = host
	if address := request.Header("x-real-ip"); address != "" {
		native.RemoteAddr = net.JoinHostPort(address, "0")
	}
	deadline, _ := yoncontext.WithDeadline(yoncontext.Background(), yoninvocation.Deadline())
	return native.WithContext(deadline)
}

func yonNativeAnswer(native *http.Response) *yonResponse {
	if native == nil {
		return yonEmpty(204)
	}
	status := native.StatusCode
	if status == 0 {
		status = 200
	}
	response := yonEmpty(status)
	for name, values := range native.Header {
		response.headers[strings.ToLower(name)] = append([]string{}, values...)
	}
	if native.Body != nil {
		body, _ := io.ReadAll(native.Body)
		native.Body.Close()
		response.body = string(body)
	}
	return response
}

func yonHTTPResponse(status int, header http.Header, body string) *http.Response {
	return &http.Response{
		Status:        strconv.Itoa(status) + " " + http.StatusText(status),
		StatusCode:    status,
		Proto:         "HTTP/1.1",
		ProtoMajor:    1,
		ProtoMinor:    1,
		Header:        header,
		Body:          io.NopCloser(strings.NewReader(body)),
		ContentLength: int64(len(body)),
	}
}

type yonRelayMessage struct {
	Route   string              `json:"route"`
	Method  string              `json:"method"`
	Headers map[string][]string `json:"headers"`
	Body    yonRequestBody      `json:"body"`
}

func yonRelayPayload(request *http.Request) []byte {
	body := []byte{}
	if request.Body != nil {
		body, _ = io.ReadAll(request.Body)
		request.Body = io.NopCloser(bytes.NewReader(body))
	}
	headers := map[string][]string{}
	for name, values := range request.Header {
		headers[strings.ToLower(name)] = values
	}
	message := yonRelayMessage{
		Route:   request.URL.RequestURI(),
		Method:  request.Method,
		Headers: headers,
		Body:    yonRequestBody{Encoding: "utf8", Data: strings.ToValidUTF8(string(body), "\uFFFD")},
	}
	payload, _ := json.Marshal(message)
	return payload
}

type YonResourceError struct{}

func (YonResourceError) Error() string {
	return "Yon managed resource operation failed."
}

type DataSource struct{}

func (DataSource) Query(sql string, parameters []any) (any, error) {
	return yonSQL(sql, parameters, "rows")
}

func (DataSource) Execute(sql string, parameters []any) (any, error) {
	return yonSQL(sql, parameters, "execute")
}

func (DataSource) Command(command map[string]any) (any, error) {
	return yonResourceCall(command)
}

func (DataSource) FindById(collection string, id string, target any) (bool, error) {
	return yonDocumentInto(map[string]any{"command": "findById", "collection": collection, "id": id}, target)
}

func (DataSource) Insert(collection string, document any) (string, error) {
	fields, err := yonDocumentFields(document)
	if err != nil {
		return "", err
	}
	var saved struct {
		ID string `json:"id"`
	}
	_, err = yonDocumentInto(map[string]any{"command": "insert", "collection": collection, "value": fields}, &saved)
	return saved.ID, err
}

func (DataSource) Patch(collection string, id string, fields any, target any) (bool, error) {
	changes, err := yonDocumentFields(fields)
	if err != nil {
		return false, err
	}
	return yonDocumentInto(map[string]any{"command": "patch", "collection": collection, "id": id, "value": changes}, target)
}

func (DataSource) DeleteById(collection string, id string) (bool, error) {
	return yonDocumentFlag(map[string]any{"command": "deleteById", "collection": collection, "id": id})
}

func (DataSource) Scroll(collection string, where map[string]any, cursor string, size int) (DocumentWindow, error) {
	command := map[string]any{"command": "scroll", "collection": collection, "where": where, "size": size}
	if cursor != "" {
		command["cursor"] = cursor
	}
	var window DocumentWindow
	_, err := yonDocumentInto(command, &window)
	return window, err
}

func (DataSource) CreateCollection(name string) error {
	_, err := yonDocumentFlag(map[string]any{"command": "createCollection", "collection": name})
	return err
}

func (DataSource) DropCollection(name string) (bool, error) {
	return yonDocumentFlag(map[string]any{"command": "dropCollection", "collection": name})
}

func (DataSource) CollectionExists(name string) (bool, error) {
	return yonDocumentFlag(map[string]any{"command": "collectionExists", "collection": name})
}

func (DataSource) Publish(topic string, payload any, key string) (string, error) {
	command := map[string]any{"operation": "queue", "command": "publish", "topic": topic, "payload": payload}
	if key != "" {
		command["key"] = key
	}
	var id string
	_, err := yonResourceInto(command, &id, yonResourceTimeout)
	return id, err
}

func (DataSource) Claim(topic string, group string, maxMessages int, leaseMs int) ([]QueueDelivery, error) {
	command := map[string]any{"operation": "queue", "command": "claim", "topic": topic, "group": group, "maxMessages": maxMessages, "leaseMs": leaseMs}
	deliveries := []QueueDelivery{}
	_, err := yonResourceInto(command, &deliveries, yonResourceTimeout)
	return deliveries, err
}

func (DataSource) Ack(topic string, group string, delivery QueueDelivery) error {
	command := map[string]any{"operation": "queue", "command": "ack", "topic": topic, "group": group, "id": delivery.ID, "receipt": delivery.Receipt}
	_, err := yonResourceInto(command, nil, yonResourceTimeout)
	return err
}

func (DataSource) Nack(topic string, group string, delivery QueueDelivery, delayMs int) error {
	command := map[string]any{"operation": "queue", "command": "nack", "topic": topic, "group": group, "id": delivery.ID, "receipt": delivery.Receipt, "delayMs": delayMs}
	_, err := yonResourceInto(command, nil, yonResourceTimeout)
	return err
}

type DocumentWindow struct {
	Content    json.RawMessage `json:"content"`
	NextCursor string          `json:"nextCursor"`
}

func (window DocumentWindow) Decode(target any) error {
	return json.Unmarshal(window.Content, target)
}

type QueueDelivery struct {
	ID      string          `json:"id"`
	Receipt string          `json:"receipt"`
	Attempt int             `json:"attempt"`
	Payload json.RawMessage `json:"payload"`
}

func (delivery QueueDelivery) Decode(target any) error {
	return json.Unmarshal(delivery.Payload, target)
}

func yonDocumentFields(value any) (map[string]any, error) {
	encoded, err := json.Marshal(value)
	if err != nil {
		return nil, YonResourceError{}
	}
	decoder := json.NewDecoder(bytes.NewReader(encoded))
	decoder.UseNumber()
	var fields map[string]any
	if decoder.Decode(&fields) != nil || fields == nil {
		return nil, YonResourceError{}
	}
	return fields, nil
}

func yonDocumentFlag(command map[string]any) (bool, error) {
	var flag bool
	_, err := yonDocumentInto(command, &flag)
	return flag, err
}

func yonDocumentInto(command map[string]any, target any) (bool, error) {
	command["operation"] = "document"
	timeout := yonResourceTimeout
	if command["command"] == "scroll" {
		timeout = yonScrollTimeout
	}
	return yonResourceInto(command, target, timeout)
}

func yonResourceInto(command map[string]any, target any, timeout time.Duration) (bool, error) {
	body, err := json.Marshal(command)
	if err != nil {
		return false, YonResourceError{}
	}
	answer, err := yonResourceExchange(body, timeout)
	if err != nil || !json.Valid(answer) {
		return false, YonResourceError{}
	}
	if string(bytes.TrimSpace(answer)) == "null" {
		return false, nil
	}
	if target != nil && json.Unmarshal(answer, target) != nil {
		return false, YonResourceError{}
	}
	return true, nil
}

func yonSQL(sql string, parameters []any, mode string) (any, error) {
	if parameters == nil {
		parameters = []any{}
	}
	command := map[string]any{"operation": "sql", "sql": sql, "parameters": parameters, "mode": mode}
	return yonResourceCall(command)
}

type RedisTemplate struct{}

func (RedisTemplate) Get(key string) (any, error) {
	return yonRedis("get", key, map[string]any{})
}

func (RedisTemplate) Set(key string, value any, ttlSeconds int) (any, error) {
	return yonRedis("set", key, map[string]any{"value": value, "ttlSeconds": ttlSeconds})
}

func (RedisTemplate) Delete(key string) (any, error) {
	return yonRedis("delete", key, map[string]any{})
}

func yonRedis(command string, key string, fields map[string]any) (any, error) {
	fields["operation"] = "redis"
	fields["command"] = command
	fields["key"] = key
	return yonResourceCall(fields)
}

const yonResourceBodyLimit = 1024 * 1024
const yonResourceHeadLimit = 8192
const yonResourceTimeout = 5 * time.Second
const yonScrollTimeout = 30 * time.Second

func yonResourceCall(command map[string]any) (any, error) {
	body, err := json.Marshal(command)
	if err != nil {
		return nil, YonResourceError{}
	}
	answer, err := yonResourceExchange(body, yonResourceTimeout)
	if err != nil || !json.Valid(answer) {
		return nil, YonResourceError{}
	}
	decoder := json.NewDecoder(bytes.NewReader(answer))
	decoder.UseNumber()
	var value any
	if decoder.Decode(&value) != nil {
		return nil, YonResourceError{}
	}
	return value, nil
}

func yonResourceExchange(body []byte, timeout time.Duration) ([]byte, error) {
	address, isLoopback := yonResourceAddress(os.Getenv("YON_RESOURCE_ENDPOINT"))
	token := os.Getenv("YON_RESOURCE_TOKEN")
	if !isLoopback || !yonResourceToken(token) || len(body) > yonResourceBodyLimit {
		return nil, YonResourceError{}
	}
	deadline := time.Now().Add(timeout)
	dialer := net.Dialer{Deadline: deadline}
	connection, err := dialer.Dial("tcp", address)
	if err != nil {
		return nil, YonResourceError{}
	}
	defer connection.Close()
	connection.SetDeadline(deadline)
	head := "POST / HTTP/1.1\r\n" +
		"Host: " + address + "\r\n" +
		"Authorization: Bearer " + token + "\r\n" +
		"Content-Type: application/json\r\n" +
		"Content-Length: " + strconv.Itoa(len(body)) + "\r\n" +
		"Connection: close\r\n\r\n"
	if _, err := connection.Write(append([]byte(head), body...)); err != nil {
		return nil, YonResourceError{}
	}
	longestResponse := yonResourceHeadLimit + yonResourceBodyLimit + 1
	response, err := io.ReadAll(io.LimitReader(connection, int64(longestResponse)))
	if err != nil {
		return nil, YonResourceError{}
	}
	return yonResourceAnswer(response)
}

func yonResourceAddress(endpoint string) (string, bool) {
	port, hasHost := strings.CutPrefix(endpoint, "http://127.0.0.1:")
	port, hasSlash := strings.CutSuffix(port, "/")
	if !hasHost || !hasSlash || len(port) == 0 || len(port) > 5 || !yonDigits(port) {
		return "", false
	}
	number, _ := strconv.Atoi(port)
	if number < 1 || number > 65535 {
		return "", false
	}
	return "127.0.0.1:" + port, true
}

func yonResourceToken(token string) bool {
	if len(token) != 64 {
		return false
	}
	for _, character := range token {
		isDigit := character >= '0' && character <= '9'
		isLetter := character >= 'a' && character <= 'f'
		if !isDigit && !isLetter {
			return false
		}
	}
	return true
}

func yonDigits(text string) bool {
	for _, character := range text {
		if character < '0' || character > '9' {
			return false
		}
	}
	return true
}

func yonResourceAnswer(response []byte) ([]byte, error) {
	split := bytes.Index(response, []byte("\r\n\r\n"))
	if split < 0 || split > yonResourceHeadLimit || !bytes.HasPrefix(response, []byte("HTTP/1.1 200 ")) {
		return nil, YonResourceError{}
	}
	head := response[:split]
	body := response[split+4:]
	if len(body) > yonResourceBodyLimit || !utf8.Valid(head) || !utf8.Valid(body) {
		return nil, YonResourceError{}
	}
	length, isValid := yonContentLength(string(head))
	if !isValid || length != len(body) {
		return nil, YonResourceError{}
	}
	return body, nil
}

func yonContentLength(head string) (int, bool) {
	length := -1
	lines := strings.Split(head, "\r\n")
	for _, line := range lines[1:] {
		name, value, hasColon := strings.Cut(line, ":")
		if !hasColon || strings.EqualFold(name, "transfer-encoding") {
			return 0, false
		}
		if !strings.EqualFold(name, "content-length") {
			continue
		}
		value = strings.TrimSpace(value)
		if length >= 0 || len(value) == 0 || !yonDigits(value) {
			return 0, false
		}
		number, err := strconv.Atoi(value)
		if err != nil {
			return 0, false
		}
		length = number
	}
	return length, length >= 0
}

type YonGatewayClient struct{}

var YonGateway = YonGatewayClient{}

func (YonGatewayClient) Call(key string, arguments string) string {
	quotedKey, _ := json.Marshal(key)
	body := `{"operation":"gateway","key":` + string(quotedKey) + `,"arguments":` + arguments + `}`
	if len(body) > yonResourceBodyLimit {
		return yonGatewayFailure("invalid")
	}
	timeLeft := yoninvocation.TimeLeft()
	if timeLeft <= 0 {
		return yonGatewayFailure("timeout")
	}
	answer, err := yonResourceExchange([]byte(body), timeLeft+time.Second)
	if err != nil && yoninvocation.TimeLeft() <= 0 {
		return yonGatewayFailure("timeout")
	}
	if err != nil || !yonGatewayAnswer(answer) {
		return yonGatewayFailure("adapter")
	}
	return string(answer)
}

func yonGatewayAnswer(answer []byte) bool {
	var fields map[string]json.RawMessage
	if json.Unmarshal(answer, &fields) != nil || fields == nil {
		return false
	}
	if _, hasResult := fields["result"]; hasResult {
		return true
	}
	var kind string
	return json.Unmarshal(fields["error"], &kind) == nil
}

func yonGatewayFailure(kind string) string {
	return `{"error":"` + kind + `"}`
}

const yonRelayStdoutLimit = 16 * 1024 * 1024
const yonRelayStderrLimit = 64 * 1024

type yonRelayOutput struct {
	limit      int
	kept       []byte
	overflowed bool
}

func (output *yonRelayOutput) Write(chunk []byte) (int, error) {
	room := output.limit - len(output.kept)
	if len(chunk) > room {
		output.overflowed = true
		output.kept = append(output.kept, chunk[:room]...)
		return len(chunk), nil
	}
	output.kept = append(output.kept, chunk...)
	return len(chunk), nil
}

func Relay(command []string, request *http.Request) *http.Response {
	if len(command) == 0 {
		return yonRelayFailure("start")
	}
	timeLeft := yoninvocation.TimeLeft()
	if timeLeft <= 0 {
		return yonRelayFailure("timeout")
	}
	stdout := &yonRelayOutput{limit: yonRelayStdoutLimit}
	stderr := &yonRelayOutput{limit: yonRelayStderrLimit}
	child := exec.Command(command[0], command[1:]...)
	child.Env = yoninvocation.RelayEnvironment()
	child.Stdin = bytes.NewReader(yonRelayPayload(request))
	child.Stdout = stdout
	child.Stderr = stderr
	if child.Start() != nil {
		return yonRelayFailure("start")
	}
	finished := make(chan error, 1)
	go func() {
		finished <- child.Wait()
	}()
	timer := time.NewTimer(timeLeft)
	defer timer.Stop()
	select {
	case <-timer.C:
		child.Process.Kill()
		return yonRelayFailure("timeout")
	case err := <-finished:
		return yonRelayResult(err, stdout, stderr)
	}
}

func yonRelayResult(err error, stdout *yonRelayOutput, stderr *yonRelayOutput) *http.Response {
	if stdout.overflowed || stderr.overflowed {
		return yonRelayFailure("overflow")
	}
	if err != nil {
		return yonRelayFailure("exit")
	}
	if !utf8.Valid(stdout.kept) {
		return yonRelayFailure("protocol")
	}
	response := yonEnvelope(stdout.kept)
	if response == nil {
		return yonRelayFailure("protocol")
	}
	return response
}

type yonRelayFailureEvent struct {
	Event     string `json:"event"`
	RequestID string `json:"request_id"`
	Category  string `json:"category"`
}

func yonRelayFailure(category string) *http.Response {
	event := yonRelayFailureEvent{Event: "handler.relay_failed", RequestID: yoninvocation.RequestID(), Category: category}
	line, _ := json.Marshal(event)
	os.Stderr.Write(append(line, '\n'))
	header := http.Header{"Content-Type": {"application/json"}}
	return yonHTTPResponse(502, header, `{"error":"Component invocation failed."}`)
}

func yonEnvelope(output []byte) *http.Response {
	var envelope map[string]json.RawMessage
	if json.Unmarshal(output, &envelope) != nil || envelope == nil {
		return nil
	}
	header := http.Header{"Content-Type": {"application/json"}}
	var headers map[string]json.RawMessage
	if json.Unmarshal(envelope["headers"], &headers) == nil && len(headers) > 0 {
		header = http.Header{}
		for name, value := range headers {
			var items []json.RawMessage
			if json.Unmarshal(value, &items) != nil || len(items) == 0 {
				header.Add(name, yonEnvelopeText(value))
				continue
			}
			for _, item := range items {
				header.Add(name, yonEnvelopeText(item))
			}
		}
	}
	return yonHTTPResponse(yonEnvelopeStatus(envelope["status"]), header, yonEnvelopeText(envelope["body"]))
}

func yonEnvelopeStatus(value json.RawMessage) int {
	var number float64
	if json.Unmarshal(value, &number) != nil || number < 1 {
		return 200
	}
	if number > 65535 {
		return 65535
	}
	return int(number)
}

func yonEnvelopeText(value json.RawMessage) string {
	var text string
	if json.Unmarshal(value, &text) == nil {
		return text
	}
	if len(value) == 0 || string(value) == "null" {
		return ""
	}
	return string(value)
}

type SocketSession struct {
	id       string
	inbox    []string
	closed   bool
	sessions []string
	answered bool
}

type SocketPeer struct {
	session *SocketSession
	id      string
}

type yonSocketFrame struct {
	Kind      string          `json:"kind"`
	RequestID string          `json:"request_id"`
	Body      *yonRequestBody `json:"body"`
	Sessions  []string        `json:"sessions"`
}

type yonSocketEndpoint interface {
	Open(session *SocketSession) error
	Message(session *SocketSession, data string) error
	Close(session *SocketSession) error
	Fail(session *SocketSession, failure error) error
}

func (session *SocketSession) ID() string {
	return session.id
}

func (session *SocketSession) Send(message any) error {
	return session.send(message, "")
}

func (session *SocketSession) Close() error {
	session.closed = true
	return session.write(map[string]any{"kind": "close"})
}

func (session *SocketSession) OpenSessions() []SocketPeer {
	session.answered = false
	if session.write(map[string]any{"kind": "sessions"}) != nil {
		return nil
	}
	for !session.answered && !session.closed {
		session.receive()
	}
	peers := []SocketPeer{}
	for _, id := range session.sessions {
		peers = append(peers, SocketPeer{session: session, id: id})
	}
	return peers
}

func (peer SocketPeer) ID() string {
	return peer.id
}

func (peer SocketPeer) Send(message any) error {
	return peer.session.send(message, peer.id)
}

func (session *SocketSession) send(message any, to string) error {
	data, failure := json.Marshal(message)
	if failure != nil {
		return failure
	}
	frame := map[string]any{"kind": "event", "body": map[string]string{"encoding": "utf8", "data": string(data)}}
	if to != "" {
		frame["to"] = to
	}
	return session.write(frame)
}

func (session *SocketSession) write(frame map[string]any) error {
	return yonWriteFrame(frame, session.id)
}

func yonWriteFrame(frame map[string]any, requestID string) error {
	frame["protocol_version"] = 1
	frame["request_id"] = requestID
	payload, failure := json.Marshal(frame)
	if failure != nil {
		return failure
	}
	length := len(payload)
	prefix := []byte{byte(length >> 24), byte(length >> 16), byte(length >> 8), byte(length)}
	_, failure = os.Stdout.Write(append(prefix, payload...))
	return failure
}

func (session *SocketSession) receive() {
	payload, open := yoninvocation.ReadFrame()
	var frame yonSocketFrame
	if !open || json.Unmarshal(payload, &frame) != nil {
		session.closed = true
		return
	}
	if frame.RequestID != session.id {
		os.Exit(70)
	}
	switch frame.Kind {
	case "message":
		if frame.Body != nil {
			session.inbox = append(session.inbox, frame.Body.Data)
		}
	case "sessions":
		session.sessions = frame.Sessions
		session.answered = true
	default:
		session.closed = true
	}
}

func (session *SocketSession) next() (string, bool) {
	for len(session.inbox) == 0 && !session.closed {
		session.receive()
	}
	if len(session.inbox) == 0 {
		return "", false
	}
	data := session.inbox[0]
	session.inbox = session.inbox[1:]
	return data, true
}

func yonSocketRead(data string, target any) error {
	return json.Unmarshal([]byte(data), target)
}

func yonSocketCall(action func() error) (failure error) {
	yoninvocation.Renew()
	defer func() {
		if recover() != nil {
			failure = errors.New("A socket method panicked.")
		}
	}()
	return action()
}

func yonSocketAttempt(session *SocketSession, endpoint yonSocketEndpoint, action func() error) {
	failure := yonSocketCall(action)
	if failure == nil {
		return
	}
	line, _ := json.Marshal(map[string]string{
		"event":      "handler.socket_failed",
		"request_id": session.id,
		"message":    failure.Error(),
	})
	os.Stderr.Write(append(line, '\n'))
	yonSocketCall(func() error { return endpoint.Fail(session, failure) })
}

func yonSocketRun(endpoint yonSocketEndpoint) {
	session := &SocketSession{id: yoninvocation.RequestID()}
	yonSocketAttempt(session, endpoint, func() error { return endpoint.Open(session) })
	for {
		data, open := session.next()
		if !open {
			break
		}
		yonSocketAttempt(session, endpoint, func() error { return endpoint.Message(session, data) })
	}
	yonSocketCall(func() error { return endpoint.Close(session) })
	os.Exit(0)
}

type yonEventStream struct {
	run func(send func(item any) error) error
}

func yonStream[T any](items yoniter.Seq[T]) yonEventStream {
	return yonEventStream{run: func(send func(item any) error) error {
		for item := range items {
			if failure := send(item); failure != nil {
				return failure
			}
		}
		return nil
	}}
}

func yonStreamWithFailures[T any](items yoniter.Seq2[T, error]) yonEventStream {
	return yonEventStream{run: func(send func(item any) error) error {
		for item, failure := range items {
			if failure != nil {
				return failure
			}
			if failure := send(item); failure != nil {
				return failure
			}
		}
		return nil
	}}
}

func (stream *yonEventStream) answer() {
	if yonStreamCall(stream) == nil {
		return
	}
	yonWriteFrame(map[string]any{
		"kind":    "response",
		"status":  500,
		"headers": map[string][]string{},
		"error":   map[string]any{"code": "TY2204", "message": "The handler stream failed.", "retryable": false},
	}, yoninvocation.RequestID())
}

func yonStreamCall(stream *yonEventStream) (failure error) {
	defer func() {
		if recover() != nil {
			failure = errors.New("A stream method panicked.")
		}
	}()
	return stream.run(yonStreamSend)
}

func yonStreamSend(item any) error {
	data, failure := json.Marshal(item)
	if failure != nil {
		return failure
	}
	frame := map[string]any{"kind": "event", "body": map[string]string{"encoding": "utf8", "data": string(data)}}
	return yonWriteFrame(frame, yoninvocation.RequestID())
}

func yonSendLoopbackThroughGuard() {
	guard := os.Getenv("YON_OUTGOING_PROXY")
	if guard == "" {
		return
	}
	address, err := yonurl.Parse(guard)
	if err != nil {
		return
	}
	transport, ok := http.DefaultTransport.(*http.Transport)
	if !ok {
		return
	}
	transport.Proxy = http.ProxyURL(address)
}

func main() {
	yonSendLoopbackThroughGuard()
	raw := yoninvocation.Request()
	request := &yonRequest{raw: raw}
	if json.Unmarshal([]byte(raw), &request.fields) != nil {
		return
	}
	response := yonDispatch(request)
	if response.stream != nil {
		response.stream.answer()
		return
	}
	answer := yonResponseFields{Status: response.status, Headers: response.headers, Body: response.body}
	output, err := json.Marshal(answer)
	if err != nil {
		return
	}
	os.Stdout.Write(output)
}
