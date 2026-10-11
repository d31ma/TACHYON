package tac

import (
	"os"
	"reflect"
	"runtime"
	"sort"
	"strconv"
	"strings"
)

type Listener struct {
	Events       []string
	Replay       bool
	Order        int64
	Condition    string
	Models       string
	Returns      string
	Asynchronous bool
	Invoke       func(value any) (any, error)
}

type Member struct {
	Name     string
	Read     func() any
	Write    func(value any) error
	Call     func(arguments []any) (any, error)
	Listener *Listener
	Hook     func() error
	Destroy  bool
	Instance any
}

type ApplicationEventPublisher struct{}

func (*ApplicationEventPublisher) PublishEvent(value any, retain bool) {
	kind := reflect.TypeOf(value)
	if kind != nil && kind.Kind() == reflect.Pointer {
		kind = kind.Elem()
	}
	if kind == nil || kind.Kind() != reflect.Struct || kind.Name() == "" {
		eventFailure()
		return
	}
	publish(kind.Name(), value, retain)
}

type service struct {
	create   func() []Member
	required []string
}

type companionRequest struct {
	operation  string
	route      string
	service    string
	owner      string
	name       string
	value      any
	arguments  []any
	properties map[string]any
}

type queuedEvent struct {
	name   string
	value  string
	retain bool
}

type delivery struct {
	listener *Listener
	value    any
}

type node struct {
	name    string
	members []Member
	needs   []*node
	holders int
	started bool
	failed  bool
}

type ownerActivation struct {
	route     string
	service   string
	owner     string
	node      *node
	members   []Member
	listeners []*Listener
	disposed  bool
	queue     []queuedEvent
	retained  map[string]string
	deferred  []delivery
}

var services = map[string]service{}
var owners = map[string]*ownerActivation{}
var shared = map[string]bool{}
var nodes = map[string]*node{}
var order []*node
var building *node
var executing *ownerActivation
var handling bool
var Emit func(payload string)
var Storage func(request string) string

func Register(name string, create func() []Member, required []string) {
	services[name] = service{create: create, required: required}
}

func Shared(name string) {
	shared[name] = true
}

func Need(name string) any {
	needed := obtain(name)
	if building != nil {
		building.needs = append(building.needs, needed)
	}
	if !start(needed) {
		panic("Native initializer failed.")
	}
	for index := range needed.members {
		if needed.members[index].Instance != nil {
			return needed.members[index].Instance
		}
	}
	return nil
}

func Invoke(text string) (reply string) {
	if handling {
		return `{"error":"Native owner is busy."}`
	}
	handling = true
	defer func() {
		if recover() != nil {
			reply = `{"error":"Native companion operation failed."}`
		}
		executing = nil
		handling = false
	}()
	reply = handle(text)
	if executing != nil {
		drain(executing)
	}
	return reply
}

func handle(text string) string {
	const startUpRequestLimit = 1048576
	const memberRequestLimit = 65536
	if len(text) > startUpRequestLimit {
		return tooLarge("request", len(text), startUpRequestLimit)
	}
	request, readable := readRequest(text)
	if !readable {
		return `{"error":"Invalid companion request."}`
	}
	if request.operation != "init" && len(text) > memberRequestLimit {
		subject := request.name
		if subject == "" {
			subject = request.operation
		}
		return tooLarge("request to "+subject, len(text), memberRequestLimit)
	}
	if request.operation == "" {
		return `{"error":"Companion request has no operation."}`
	}
	if request.operation == "host-dispose" {
		disposeAll()
		return `{"value":null}`
	}
	if request.route == "" {
		return `{"error":"Missing companion route.","code":"TY_NATIVE_ROUTE"}`
	}
	if request.service == "" {
		return `{"error":"Missing companion name.","code":"TY_NATIVE_SERVICE"}`
	}
	if !isOwnerID(request.owner) {
		return `{"error":"Missing native owner identity.","code":"TY_NATIVE_OWNER"}`
	}
	return handleOwned(request)
}

func readRequest(text string) (*companionRequest, bool) {
	parsed, err := parseJSON(text)
	record, isRecord := parsed.(map[string]any)
	if err != nil || !isRecord {
		return nil, false
	}
	request := &companionRequest{value: record["value"]}
	request.operation, _ = record["op"].(string)
	request.route, _ = record["route"].(string)
	request.service, _ = record["service"].(string)
	request.owner, _ = record["owner"].(string)
	request.name, _ = record["name"].(string)
	request.arguments, _ = record["args"].([]any)
	request.properties, _ = record["props"].(map[string]any)
	return request, true
}

func handleOwned(request *companionRequest) string {
	if request.operation == "init" {
		failed := activate(request)
		if failed != "" {
			return failed
		}
	}
	activation := owners[request.owner]
	if activation == nil || activation.route != request.route || activation.service != request.service {
		return `{"error":"Stale native owner.","code":"TY_NATIVE_OWNER"}`
	}
	if request.operation == "dispose" {
		disposeOwner(request.owner)
		return `{"value":null}`
	}
	executing = activation
	if request.operation == "init" {
		return describe(activation.members)
	}
	if request.name == "" {
		return `{"error":"Companion request has no member name."}`
	}
	member := findMember(activation.members, request.name)
	if member == nil {
		return errorAnswer("Unknown companion member: " + request.name)
	}
	if request.operation == "event" {
		return receive(request, member)
	}
	return useMember(request, member)
}

func receive(request *companionRequest, member *Member) string {
	if member.Listener == nil {
		return errorAnswer("Unknown companion listener: " + request.name)
	}
	result, err := member.Listener.Invoke(request.value)
	if err != nil {
		return errorAnswer("Native event listener failed.")
	}
	if !member.Listener.Asynchronous && result != nil {
		republish(result, member.Listener.Returns)
	}
	return `{"value":null}`
}

func activate(request *companionRequest) string {
	existing := owners[request.owner]
	if existing != nil && existing.route == request.route && existing.service == request.service {
		return ""
	}
	if existing != nil {
		disposeOwner(request.owner)
	}
	known, isKnown := services[request.service]
	if !isKnown {
		return `{"error":"Unknown native service.","code":"TY_NATIVE_SERVICE"}`
	}
	if len(owners) >= 256 {
		return `{"error":"Too many native activations."}`
	}
	activation := &ownerActivation{
		route:    request.route,
		service:  request.service,
		owner:    request.owner,
		retained: map[string]string{},
	}
	owners[request.owner] = activation
	executing = activation
	attach(activation, known)
	assignProperties(activation, known.required, request.properties)
	if !runInitializers(activation) {
		disposeOwner(request.owner)
		return `{"error":"Native initializer failed."}`
	}
	drain(activation)
	return ""
}

func attach(activation *ownerActivation, known service) {
	defer func() {
		if recovered := recover(); recovered != nil {
			disposeOwner(activation.owner)
			panic(recovered)
		}
	}()
	if shared[activation.service] {
		activation.node = obtain(activation.service)
	} else {
		activation.node = build(activation.service, known.create)
		activation.node.holders = 1
	}
	members := activation.node.members
	activation.members = members
	for index := range members {
		if members[index].Listener != nil {
			activation.listeners = append(activation.listeners, members[index].Listener)
		}
	}
	sort.SliceStable(activation.listeners, func(left int, right int) bool {
		return activation.listeners[left].Order < activation.listeners[right].Order
	})
}

func obtain(name string) *node {
	existing := nodes[name]
	if existing != nil {
		existing.holders++
		return existing
	}
	known, isKnown := services[name]
	if !isKnown {
		panic("Unknown native service.")
	}
	made := build(name, known.create)
	made.holders = 1
	nodes[name] = made
	order = append(order, made)
	return made
}

func build(name string, create func() []Member) *node {
	made := &node{name: name}
	outer := building
	building = made
	defer func() {
		building = outer
		if recovered := recover(); recovered != nil {
			releaseNeeds(made)
			panic(recovered)
		}
	}()
	made.members = create()
	return made
}

func start(started *node) bool {
	if started.started {
		return true
	}
	started.started = true
	for index := range started.members {
		member := &started.members[index]
		if member.Hook == nil || member.Destroy {
			continue
		}
		if member.Hook() != nil {
			started.failed = true
			return false
		}
	}
	return true
}

func release(held *node) {
	held.holders--
	parks := shared[held.name] && !held.failed && len(owners) > 0 && nodes[held.name] == held
	if held.holders > 0 || parks {
		return
	}
	finish(held)
}

func finish(held *node) {
	if nodes[held.name] == held {
		delete(nodes, held.name)
	}
	if held.started {
		for index := range held.members {
			member := &held.members[index]
			if member.Hook == nil || !member.Destroy {
				continue
			}
			if member.Hook() != nil {
				eventFailure()
			}
		}
	}
	releaseNeeds(held)
}

func sweep() {
	if len(owners) > 0 {
		return
	}
	for index := len(order) - 1; index >= 0; index-- {
		parked := order[index]
		if parked.holders == 0 && nodes[parked.name] == parked {
			finish(parked)
		}
	}
	order = nil
}

func releaseNeeds(held *node) {
	needs := held.needs
	held.needs = nil
	for index := len(needs) - 1; index >= 0; index-- {
		release(needs[index])
	}
}

func assignProperties(activation *ownerActivation, required []string, properties map[string]any) {
	for key, value := range properties {
		if !contains(required, key) && !contains(required, "*") {
			continue
		}
		member := findMember(activation.members, key)
		if member != nil && member.Write != nil {
			_ = member.Write(value)
		}
	}
}

func runInitializers(activation *ownerActivation) bool {
	return start(activation.node)
}

func describe(members []Member) string {
	fields := []any{}
	methods := []any{}
	listeners := []any{}
	for index := range members {
		if members[index].Read != nil {
			fields = append(fields, members[index].Name)
		} else if members[index].Call != nil {
			methods = append(methods, members[index].Name)
		} else if members[index].Listener != nil {
			listeners = append(listeners, describeListener(members[index].Name, members[index].Listener))
		}
	}
	return valueAnswer(map[string]any{"fields": fields, "methods": methods, "listeners": listeners})
}

func describeListener(key string, listener *Listener) map[string]any {
	events := []any{}
	for _, event := range listener.Events {
		events = append(events, event)
	}
	var condition any
	if listener.Condition != "" {
		condition, _ = parseJSON(listener.Condition)
	}
	var models any
	if listener.Models != "" {
		models, _ = parseJSON(listener.Models)
	}
	return map[string]any{
		"models":       models,
		"key":          key,
		"events":       events,
		"replay":       listener.Replay,
		"order":        listener.Order,
		"asynchronous": listener.Asynchronous,
		"condition":    condition,
	}
}

func useMember(request *companionRequest, member *Member) string {
	operation := request.operation
	if operation == "get" && member.Read != nil {
		return valueAnswer(member.Read())
	}
	if operation == "set" && member.Write != nil {
		if member.Write(request.value) != nil {
			return errorAnswer("Companion field cannot hold this value: " + request.name)
		}
		return `{"value":null}`
	}
	if operation == "set" && member.Read != nil {
		return errorAnswer("Companion field is read-only: " + request.name)
	}
	if operation == "call" && member.Call != nil {
		value, err := member.Call(request.arguments)
		if err != nil {
			return codedErrorAnswer(err)
		}
		return valueAnswer(value)
	}
	return errorAnswer("Companion member does not support " + operation + ": " + request.name)
}

func findMember(members []Member, name string) *Member {
	for index := range members {
		if members[index].Name == name {
			return &members[index]
		}
	}
	return nil
}

func contains(values []string, wanted string) bool {
	for _, value := range values {
		if value == wanted {
			return true
		}
	}
	return false
}

func valueAnswer(value any) string {
	encoded, err := encodeJSON(value)
	if err != nil {
		return errorAnswer("Companion answer is not JSON.")
	}
	return `{"value":` + encoded + `}`
}

func tooLarge(subject string, size int, limit int) string {
	return errorAnswer("The " + subject + " is " + strconv.Itoa(size) + " bytes. The limit is " +
		strconv.Itoa(limit) + " bytes. Pass less data to the service.")
}

func codedErrorAnswer(err error) string {
	coded, hasCode := err.(interface{ BehaviorCode() string })
	if !hasCode {
		return errorAnswer(err.Error())
	}
	return `{"error":` + quote(err.Error()) + `,"code":` + quote(coded.BehaviorCode()) + `}`
}

func errorAnswer(message string) string {
	return `{"error":` + quote(message) + `}`
}

func isOwnerID(text string) bool {
	if len(text) != 36 {
		return false
	}
	for index := 0; index < len(text); index++ {
		character := text[index]
		isHyphenPosition := index == 8 || index == 13 || index == 18 || index == 23
		if isHyphenPosition {
			if character != '-' {
				return false
			}
			continue
		}
		isDigit := character >= '0' && character <= '9'
		isLower := character >= 'a' && character <= 'f'
		isUpper := character >= 'A' && character <= 'F'
		if !isDigit && !isLower && !isUpper {
			return false
		}
	}
	return true
}

func eventFailure() {
	_, _ = os.Stderr.WriteString("Tachyon native event delivery failed.\n")
}

func publish(name string, value any, retain bool) {
	owner := executing
	if owner == nil || owner.disposed {
		eventFailure()
		return
	}
	encoded, err := encodeJSON(value)
	if err != nil || len(encoded) > 65536 || len(owner.queue) >= 128 {
		eventFailure()
		return
	}
	owner.queue = append(owner.queue, queuedEvent{name: name, value: encoded, retain: retain})
}

func drain(owner *ownerActivation) {
	delivered := 0
	for !owner.disposed {
		if len(owner.deferred) > 0 {
			pending := owner.deferred
			owner.deferred = nil
			for _, waiting := range pending {
				deliver(waiting.listener, waiting.value, false)
			}
			continue
		}
		if len(owner.queue) == 0 {
			return
		}
		next := owner.queue[0]
		owner.queue = owner.queue[1:]
		if delivered == 128 {
			owner.queue = nil
			eventFailure()
			return
		}
		delivered++
		if runtime.GOOS == "js" {
			sendEvent(owner, next)
			continue
		}
		if next.retain && !remember(owner, next) {
			eventFailure()
			continue
		}
		notifyListeners(owner, next)
		sendEvent(owner, next)
	}
}

func remember(owner *ownerActivation, event queuedEvent) bool {
	_, known := owner.retained[event.name]
	if !known && len(owner.retained) == 32 {
		return false
	}
	owner.retained[event.name] = event.value
	return true
}

func notifyListeners(owner *ownerActivation, event queuedEvent) {
	value, _ := parseJSON(event.value)
	for _, listener := range owner.listeners {
		if !contains(listener.Events, event.name) {
			continue
		}
		if listener.Models != "" && !modelFits(listener.Models, event.name, value) {
			eventFailure()
			continue
		}
		if listener.Condition != "" && !conditionHolds(listener.Condition, value, event.name) {
			continue
		}
		if !listener.Asynchronous {
			deliver(listener, value, true)
			continue
		}
		if len(owner.deferred) >= 128 {
			eventFailure()
			continue
		}
		owner.deferred = append(owner.deferred, delivery{listener: listener, value: value})
	}
}

func deliver(listener *Listener, value any, mayRepublish bool) {
	result, err := listener.Invoke(value)
	if err != nil {
		eventFailure()
		return
	}
	if mayRepublish && result != nil {
		republish(result, listener.Returns)
	}
}

func republish(result any, returns string) {
	if returns == "" {
		return
	}
	encoded, err := encodeJSON(result)
	if err != nil {
		return
	}
	decoded, _ := parseJSON(encoded)
	items, isList := decoded.([]any)
	if !isList {
		items = []any{decoded}
	}
	if len(items) > 32 {
		items = items[:32]
	}
	for _, item := range items {
		if item != nil {
			publish(returns, item, false)
		}
	}
}

func modelFits(models string, shape string, value any) bool {
	declared, err := parseJSON(models)
	if err != nil {
		return false
	}
	return fits(declared, shape, value)
}

func fits(models any, shape string, value any) bool {
	if inner, optional := strings.CutPrefix(shape, "?"); optional {
		return value == nil || fits(models, inner, value)
	}
	if item, isList := strings.CutPrefix(shape, "[]"); isList {
		items, isArray := value.([]any)
		if !isArray {
			return false
		}
		for _, entry := range items {
			if !fits(models, item, entry) {
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
	declared, _ := models.(map[string]any)
	model, isModel := declared[shape].(map[string]any)
	object, isObject := value.(map[string]any)
	if !isModel || !isObject {
		return false
	}
	fields, _ := model["fields"].(map[string]any)
	if open, _ := model["open"].(bool); !open {
		for key := range object {
			if _, known := fields[key]; !known {
				return false
			}
		}
	}
	for name, declaredShape := range fields {
		field, _ := declaredShape.(string)
		inner, required := strings.CutPrefix(field, "!")
		found, present := object[name]
		if !present {
			if required {
				return false
			}
			continue
		}
		if !fits(models, inner, found) {
			return false
		}
	}
	return true
}

func sendEvent(owner *ownerActivation, event queuedEvent) {
	if Emit == nil {
		return
	}
	payload := `{"route":` + quote(owner.route) +
		`,"service":` + quote(owner.service) +
		`,"owner":` + quote(owner.owner) +
		`,"name":` + quote(event.name) +
		`,"value":` + event.value +
		`,"retain":` + strconv.FormatBool(event.retain) + `}`
	Emit(payload)
}

func disposeActivation(owner *ownerActivation) {
	owner.disposed = true
	owner.queue = nil
	owner.retained = map[string]string{}
	owner.deferred = nil
	held := owner.node
	owner.node = nil
	if held != nil {
		release(held)
	}
}

func disposeOwner(id string) {
	cancelHTTP(id)
	owner := owners[id]
	delete(owners, id)
	if executing != nil && executing.owner == id {
		executing = nil
	}
	if owner != nil {
		disposeActivation(owner)
	}
	sweep()
}

func disposeAll() {
	active := owners
	owners = map[string]*ownerActivation{}
	executing = nil
	for _, owner := range active {
		disposeActivation(owner)
	}
	sweep()
}
