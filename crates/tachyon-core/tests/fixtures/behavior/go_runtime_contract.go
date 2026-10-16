package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

const behaviorModuleID = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
const behaviorEnvironment = "yon"

func behaviorStorageHost() func(string) string {
	return nil
}

func behaviorBrowserHost() func(string) string {
	return nil
}

type Memory struct {
	lock           sync.Mutex
	entries        map[string]BehaviorCacheEntry
	authorizations atomic.Int64
	reads          atomic.Int64
}

func (memory *Memory) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	read := memory.reads.Add(1)
	expect(memory.authorizations.Load() >= read, "authorization runs before every read, including hits")
	expect(strings.Contains(key, behaviorModuleID), "the cache key names the module")
	memory.lock.Lock()
	defer memory.lock.Unlock()
	entry, present := memory.entries[key]
	if !present {
		return nil, nil
	}
	return &entry, nil
}

func (memory *Memory) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	if err := write.Execution.Check(); err != nil {
		return err
	}
	if write.Execution.Partition == "write-fails" {
		return behaviorInvalid()
	}
	memory.lock.Lock()
	defer memory.lock.Unlock()
	memory.entries[key] = BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (memory *Memory) Delete(key string, execution *BehaviorExecution) error {
	if err := execution.Check(); err != nil {
		return err
	}
	memory.lock.Lock()
	defer memory.lock.Unlock()
	delete(memory.entries, key)
	return nil
}

func (memory *Memory) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	expect(request.Redirect == "error", "redirects fail")
	expect(request.Credentials == "omit", "credentials are omitted")
	expect(request.MaxResponseBytes == 1048576, "responses are bounded")
	expect(request.Path == "/products", "the declared path is used")
	return "transport", nil
}

func (memory *Memory) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	expect(command.Table == "products", "the entity table is used")
	if command.Query == nil {
		return command.Operation, nil
	}
	return memory.executeQuery(command, execution)
}

func (memory *Memory) executeQuery(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	expect(execution.Partition != "query-denied", "a denied query reached the adapter")
	expect(command.Operation == "query", "queries run as query commands")
	expect(command.Value == nil && command.Page == nil && command.Id == nil, "queries carry only the query")
	if execution.Partition == "query-bad-result" {
		return "not a query result", nil
	}
	if execution.Partition == "query-too-many" {
		rows := []any{}
		for index := 0; index < 101; index++ {
			rows = append(rows, map[string]any{})
		}
		return rows, nil
	}
	query := command.Query.(map[string]any)
	switch query["kind"] {
	case "find":
		idAscending := []any{map[string]any{"field": "id", "direction": "asc"}}
		expect(same(query["orderBy"], idAscending), "queries order by id")
		return []any{map[string]any{"id": "one"}}, nil
	case "count":
		return 1, nil
	case "exists":
		return true, nil
	}
	return nil, behaviorInvalid()
}

type Config struct{}

func (Config) behaviorConditions() BehaviorConfigurationConditions {
	return BehaviorConfigurationConditions{}
}

func (Config) behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	memory := &Memory{entries: map[string]BehaviorCacheEntry{}}
	config := NewBehaviorConfiguration()
	config.Caches["products"] = memory
	config.Http["catalog"] = memory
	config.Entities["data"] = memory
	config.Authorize = func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error) {
		if err := execution.Check(); err != nil {
			return false, err
		}
		memory.authorizations.Add(1)
		return execution.Partition != "denied", nil
	}
	config.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		if err := execution.Check(); err != nil {
			return false, err
		}
		expect(command.Operation == "query", "entity authorization sees the query")
		expect(command.Entity == "Product", "entity authorization sees the entity")
		if execution.Partition == "query-timeout" {
			time.Sleep(10 * time.Millisecond)
		}
		return execution.Partition != "query-denied", nil
	}
	return config, nil
}

type Product struct{}

func (Product) behaviorEntity() BehaviorEntityMetadata {
	return BehaviorEntityMetadata{Name: "Product", Adapter: "data", Table: "products", Id: "id"}
}

func expect(condition bool, message string) {
	if !condition {
		panic(message)
	}
}

func expectCode(err error, code string, message string) {
	if BehaviorErrorCode(err) != code {
		panic(fmt.Sprintf("%s: expected %s, found %v", message, code, err))
	}
}

func must[T any](value T, err error) T {
	if err != nil {
		panic(err)
	}
	return value
}

func same(left any, right any) bool {
	leftText, leftError := json.Marshal(left)
	rightText, rightError := json.Marshal(right)
	return leftError == nil && rightError == nil && string(leftText) == string(rightText)
}

func emptyObject() BehaviorValue {
	return map[string]any{}
}

func createContext(partition string) (*BehaviorContext, error) {
	return BehaviorsCreate[Config](NewBehaviorOptions(partition, NewBehaviorSignal()))
}

func policy() BehaviorMethodPolicy {
	return BehaviorMethodPolicy{
		Owner:    "Catalog",
		Name:     "list",
		Cache:    &BehaviorCachePolicy{Name: "products", TtlMs: 1000, MaxBytes: 1024, MaxEntries: 10},
		KeyArgs:  []int{0},
		Pageable: &BehaviorPagePolicy{Argument: 0, DefaultSize: 20, MaxSize: 100, Sort: []string{"id"}},
	}
}

func plainPolicy() BehaviorMethodPolicy {
	plain := policy()
	plain.Cache = nil
	plain.KeyArgs = nil
	plain.Pageable = nil
	return plain
}

func cancellationRetryPolicy() BehaviorMethodPolicy {
	result := plainPolicy()
	result.Retry = &BehaviorRetryPolicy{MaxRetries: 2, Multiplier: 1}
	return result
}

func checkCancellationDoesNotRetry(failure error, code string) {
	context := must(createContext("cancelled-retry"))
	defer context.Dispose()
	calls := 0
	_, err := context.Invoke(cancellationRetryPolicy(), nil, func([]BehaviorValue) (BehaviorValue, error) {
		calls++
		return nil, failure
	})
	expectCode(err, code, "a cancelled invocation keeps its error code")
	expect(calls == 1, "a cancelled invocation must not retry: "+code)
}

func checkRetryableFailureStillRetries() {
	context := must(createContext("network-retry"))
	defer context.Dispose()
	calls := 0
	value, err := context.Invoke(cancellationRetryPolicy(), nil, func([]BehaviorValue) (BehaviorValue, error) {
		calls++
		if calls < 3 {
			return nil, behaviorError("HTTP_NETWORK", "temporary network failure")
		}
		return "recovered", nil
	})
	expect(err == nil && value == "recovered", "a retryable failure can recover")
	expect(calls == 3, "a retryable failure keeps the configured attempts")
}

func cancellationRetryContracts() {
	_, browserFailure := behaviorBrowserAnswer(`{"error":{"code":"CANCELLED"}}`)
	expectCode(browserFailure, "CANCELLED", "browser cancellation uses its wire code")
	for _, failure := range []error{browserFailure, fmt.Errorf("wrapped cancellation: %w", browserFailure)} {
		checkCancellationDoesNotRetry(failure, "CANCELLED")
	}
	failure := behaviorError("cancelled", "cancelled operation")
	for _, cancellation := range []error{failure, fmt.Errorf("wrapped cancellation: %w", failure)} {
		checkCancellationDoesNotRetry(cancellation, "cancelled")
	}
	checkRetryableFailureStillRetries()
}

func mutationPolicy(operation string) BehaviorMethodPolicy {
	result := policy()
	result.Name = "update"
	result.KeyArgs = nil
	result.Pageable = nil
	result.Mutation = &BehaviorCacheMutationPolicy{Operation: operation, Method: "list", Arguments: []int{0}, KeyArgs: []int{0}, Pageable: policy().Pageable}
	return result
}

func missed([]BehaviorValue) (BehaviorValue, error) {
	panic("this body must not run")
}

func checkCachePutAndEvict(context *BehaviorContext) {
	calls := 0
	for attempt := 0; attempt < 2; attempt++ {
		must(context.Invoke(mutationPolicy("cache:put"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
			calls++
			return "replacement", nil
		}))
	}
	expect(calls == 2, "CachePut always executes the body")
	read := func() BehaviorValue {
		return must(context.Invoke(policy(), []BehaviorValue{emptyObject()}, missed))
	}
	expect(read() == "replacement", "CachePut stored the replacement")
	_, err := context.Invoke(mutationPolicy("cache:put"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		return strings.Repeat("x", 1024), nil
	})
	expect(err != nil, "an oversized CachePut fails")
	expect(read() == "replacement", "an oversized CachePut keeps the entry")
	_, err = context.Invoke(mutationPolicy("cache:evict"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		return nil, behaviorInvalid()
	})
	expect(err != nil, "a failed CacheEvict fails")
	expect(read() == "replacement", "a failed CacheEvict keeps the entry")
	_, err = context.Invoke(mutationPolicy("cache:evict"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		return math.NaN(), nil
	})
	expect(err != nil, "a NaN result fails")
	expect(read() == "replacement", "a NaN result keeps the entry")
	must(context.Invoke(mutationPolicy("cache:evict"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		return true, nil
	}))
	fresh := must(context.Invoke(policy(), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		return "fresh", nil
	}))
	expect(fresh == "fresh", "CacheEvict removed the entry")
}

func checkDeniedMutations() {
	denied := must(createContext("denied"))
	for _, operation := range []string{"cache:put", "cache:evict"} {
		_, err := denied.Invoke(mutationPolicy(operation), []BehaviorValue{emptyObject()}, missed)
		expectCode(err, "unauthorized", "an unauthorized mutation")
	}
}

func checkInvalidMutationPolicies(context *BehaviorContext) {
	invalid := mutationPolicy("cache:put")
	invalid.Mutation.Arguments = []int{2}
	_, err := context.Invoke(invalid, []BehaviorValue{emptyObject()}, missed)
	expect(err != nil, "an invalid mapping fails")
	invalid = mutationPolicy("cache:put")
	invalid.KeyArgs = []int{0}
	_, err = context.Invoke(invalid, []BehaviorValue{emptyObject()}, missed)
	expect(err != nil, "a Cacheable mutation fails")
}

func checkFailedCacheWrite() {
	failing := must(createContext("write-fails"))
	sideEffects := 0
	_, err := failing.Invoke(mutationPolicy("cache:put"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		sideEffects++
		return "value", nil
	})
	expectCode(err, "adapter", "a failed cache write")
	expect(sideEffects == 1, "no retry after a write failure")
}

func checkInterruptedMutations() {
	signal := NewBehaviorSignal()
	aborted := must(BehaviorsCreate[Config](NewBehaviorOptions("abort-mutation", signal)))
	_, err := aborted.Invoke(mutationPolicy("cache:put"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		signal.Abort()
		return "value", nil
	})
	expectCode(err, "aborted", "an aborted mutation")
	options := NewBehaviorOptions("deadline-mutation", NewBehaviorSignal())
	options.TimeoutMs = 5
	deadline := must(BehaviorsCreate[Config](options))
	_, err = deadline.Invoke(mutationPolicy("cache:evict"), []BehaviorValue{emptyObject()}, func([]BehaviorValue) (BehaviorValue, error) {
		time.Sleep(10 * time.Millisecond)
		return true, nil
	})
	expectCode(err, "timeout", "a late mutation")
}

func mutationContracts() {
	context := must(createContext("mutations"))
	checkCachePutAndEvict(context)
	checkDeniedMutations()
	checkInvalidMutationPolicies(context)
	checkFailedCacheWrite()
	checkInterruptedMutations()
}

func queryPolicy() BehaviorQueryPolicy {
	return BehaviorQueryPolicy{
		Entity:      Product{}.behaviorEntity(),
		Fields:      []string{"id", "status"},
		DefaultSize: 20,
		MaxSize:     100,
		Operation:   "find",
		Predicates:  []BehaviorQueryPredicate{{Field: "status", Operator: "eq", Argument: 0, HasArgument: true}},
		OrderBy:     []BehaviorQuerySort{{Field: "id", Direction: "asc"}},
	}
}

func active() []BehaviorValue {
	return []BehaviorValue{"active"}
}

func checkQueryResults(context *BehaviorContext) {
	rows := must(context.Query(queryPolicy(), active()))
	found, isList := rows.([]any)
	expect(isList && len(found) == 1, "a find returns one row")
	nearLimit := []BehaviorValue{strings.Repeat("x", 65460)}
	normalized := must(behaviorQuery(queryPolicy(), nearLimit))
	_, err := behaviorEncode(normalized.(map[string]any)["where"], 65536)
	expect(err == nil, "the predicates fit their budget")
	whole := must(behaviorEncode(normalized, 1048576))
	expect(len(whole) > 65536, "the whole query is larger than the predicate budget")
	_, err = context.Query(queryPolicy(), nearLimit)
	expect(err == nil, "query metadata must not consume the separate binding budget")
}

func checkCountAndExists(context *BehaviorContext) {
	for _, operation := range []string{"count", "exists"} {
		counted := queryPolicy()
		counted.Operation = operation
		counted.OrderBy = nil
		var expected BehaviorValue = 1.0
		if operation == "exists" {
			expected = true
		}
		expect(must(context.Query(counted, active())) == expected, "count and exists answer")
		broken := must(createContext("query-bad-result"))
		_, err := broken.Query(counted, active())
		expect(err != nil, "a bad count or exists result fails")
	}
}

type MissingAuthorization struct{}

func (MissingAuthorization) behaviorConditions() BehaviorConfigurationConditions {
	return BehaviorConfigurationConditions{}
}

func (MissingAuthorization) behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}

func checkRejectedQueryPartitions() {
	for _, partition := range []string{"query-bad-result", "query-too-many", "query-denied"} {
		other := must(createContext(partition))
		_, err := other.Query(queryPolicy(), active())
		expect(err != nil, "partition "+partition+" fails")
	}
	missing := must(BehaviorsCreate[MissingAuthorization](NewBehaviorOptions("missing-query-auth", NewBehaviorSignal())))
	_, err := missing.Query(queryPolicy(), active())
	expect(err != nil, "a query without entity authorization fails")
}

func checkQueryBindings(context *BehaviorContext) {
	for _, value := range []BehaviorValue{math.NaN(), map[string]any{"sql": "untrusted"}, strings.Repeat("x", 65536)} {
		_, err := context.Query(queryPolicy(), []BehaviorValue{value})
		expect(err != nil, "an unsafe binding fails")
	}
	inPolicy := queryPolicy()
	inPolicy.Predicates[0].Operator = "in"
	hundred := []any{}
	for index := 0; index < 100; index++ {
		hundred = append(hundred, "active")
	}
	_, err := context.Query(inPolicy, []BehaviorValue{hundred})
	expect(err == nil, "an in list of 100 works")
	_, err = context.Query(inPolicy, []BehaviorValue{append(hundred, "active")})
	expect(err != nil, "an in list of 101 fails")
}

func checkQueryPages(context *BehaviorContext) {
	pagePolicy := queryPolicy()
	pagePolicy.PageArgument = 1
	pagePolicy.HasPageArgument = true
	unknownSort := map[string]any{"sort": []any{"status"}}
	_, err := context.Query(pagePolicy, []BehaviorValue{"active", unknownSort})
	expect(err != nil, "a query page cannot sort")
	_, err = context.Query(pagePolicy, []BehaviorValue{"active", map[string]any{"size": 101}})
	expect(err != nil, "an oversized page fails")
	_, err = context.Query(pagePolicy, []BehaviorValue{"active", map[string]any{"size": 1}})
	expect(err == nil, "a single row page works")
}

func checkInvalidQueryCalls(context *BehaviorContext) {
	_, err := context.Query(queryPolicy(), []BehaviorValue{})
	expect(err != nil, "a missing binding fails")
	_, err = context.Query(queryPolicy(), []BehaviorValue{"active", "unused"})
	expect(err != nil, "an unused binding fails")
	invalid := queryPolicy()
	invalid.Predicates[0].Field = "unknown"
	_, err = context.Query(invalid, active())
	expect(err != nil, "an unknown field fails")
	invalid = queryPolicy()
	invalid.Operation = "update"
	_, err = context.Query(invalid, active())
	expect(err != nil, "an unknown operation fails")
}

func checkInterruptedQueries(context *BehaviorContext) {
	options := NewBehaviorOptions("query-timeout", NewBehaviorSignal())
	options.TimeoutMs = 5
	deadline := must(BehaviorsCreate[Config](options))
	_, err := deadline.Query(queryPolicy(), active())
	expectCode(err, "timeout", "a late query")
	signal := NewBehaviorSignal()
	cancelled := must(BehaviorsCreate[Config](NewBehaviorOptions("query-cancelled", signal)))
	signal.Abort()
	_, err = cancelled.Query(queryPolicy(), active())
	expectCode(err, "aborted", "an aborted query")
	context.Dispose()
	_, err = context.Query(queryPolicy(), active())
	expectCode(err, "disposed", "a disposed query")
}

func queryContracts() {
	context := must(createContext("queries"))
	checkQueryResults(context)
	checkCountAndExists(context)
	checkRejectedQueryPartitions()
	checkQueryBindings(context)
	checkQueryPages(context)
	checkInvalidQueryCalls(context)
	checkInterruptedQueries(context)
}

type Settings struct{}

func (Settings) behaviorProperties() BehaviorPropertiesPolicy {
	return BehaviorPropertiesPolicy{
		Prefix: "YON_APP_",
		Fields: []BehaviorPropertyField{
			{Field: "title", Name: "TITLE", Kind: "string", Default: "catalog"},
			{Field: "enabled", Name: "ENABLED", Kind: "boolean", Default: false},
			{Field: "limit", Name: "LIMIT", Kind: "integer", Default: float64(20), Min: 1, HasMin: true, Max: 100, HasMax: true},
			{Field: "ratio", Name: "RATIO", Kind: "number", Default: 0.5, Min: 0, HasMin: true, Max: 1, HasMax: true},
		},
	}
}

type ConditionalConfig struct{}

var conditionalConfigureCalls atomic.Int64

func (ConditionalConfig) behaviorConditions() BehaviorConfigurationConditions {
	return BehaviorConfigurationConditions{
		Profiles: []string{"development", "test"},
		Property: &BehaviorPropertyCondition{Name: "YON_FEATURE_ENABLED", HavingValue: "true", MatchIfMissing: false},
	}
}

func (ConditionalConfig) behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	conditionalConfigureCalls.Add(1)
	return NewBehaviorConfiguration(), nil
}

func propertyValues() map[string]string {
	return map[string]string{
		"YON_APP_TITLE":   "カタログ",
		"YON_APP_ENABLED": "true",
		"YON_APP_LIMIT":   "42",
		"YON_APP_RATIO":   "1e-1",
	}
}

func checkPropertyBinding(values map[string]string) {
	defaults := must(BehaviorsProperties[Settings](map[string]string{})).(map[string]any)
	expect(defaults["limit"] == 20.0, "the default limit is bound")
	bound := must(BehaviorsProperties[Settings](values)).(map[string]any)
	expect(bound["title"] == "カタログ", "the title is bound")
	expect(bound["enabled"] == true, "the flag is bound")
	expect(bound["limit"] == 42.0, "the limit is bound")
	expect(bound["ratio"] == 0.1, "the ratio is bound")
}

func checkRejectedPropertyValues(values map[string]string) {
	rejected := [][2]string{
		{"YON_APP_LIMIT", "1.0"}, {"YON_APP_LIMIT", "1e1"}, {"YON_APP_LIMIT", "+1"}, {"YON_APP_LIMIT", " 1"},
		{"YON_APP_LIMIT", "01"}, {"YON_APP_LIMIT", "-0"}, {"YON_APP_LIMIT", "101"}, {"YON_APP_LIMIT", "9007199254740992"},
		{"YON_APP_RATIO", "NaN"}, {"YON_APP_RATIO", ".1"}, {"YON_APP_RATIO", "1."}, {"YON_APP_RATIO", "+0.1"},
		{"YON_APP_RATIO", "Infinity"}, {"YON_APP_RATIO", "1e999"}, {"YON_APP_ENABLED", "TRUE"}, {"YON_APP_ENABLED", "1"},
		{"YON_APP_UNKNOWN", "redacted-secret"}, {"TAC_APP_TITLE", "redacted-secret"}, {"YON_bad", "redacted-secret"},
	}
	for _, entry := range rejected {
		data := map[string]string{}
		for key, value := range values {
			data[key] = value
		}
		data[entry[0]] = entry[1]
		_, err := BehaviorsProperties[Settings](data)
		expectCode(err, "configuration-properties", "property "+entry[0]+"="+entry[1])
		expect(!strings.Contains(err.Error(), entry[1]) || len(entry[1]) < 5, "errors never echo the value")
	}
}

func checkPropertySizeLimits() {
	longTitle := map[string]string{"YON_APP_TITLE": strings.Repeat("é", 2049)}
	tooManyKeys := map[string]string{}
	for index := 0; index < 257; index++ {
		tooManyKeys[fmt.Sprintf("YON_VALUE_%d", index)] = ""
	}
	tooManyBytes := map[string]string{}
	for index := 0; index < 20; index++ {
		tooManyBytes[fmt.Sprintf("YON_VALUE_%d", index)] = strings.Repeat("x", 4096)
	}
	for _, data := range []map[string]string{longTitle, tooManyKeys, tooManyBytes} {
		_, err := BehaviorsProperties[Settings](data)
		expect(err != nil, "oversized properties fail")
	}
}

func checkInvalidPropertyPolicies() {
	noValues := map[string]string{}
	required := Settings{}.behaviorProperties()
	required.Fields[0].Default = nil
	_, err := behaviorBindProperties(required, noValues)
	expect(err != nil, "a missing required property fails")
	invalid := Settings{}.behaviorProperties()
	invalid.Fields[0].Default = true
	_, err = behaviorBindProperties(invalid, noValues)
	expect(err != nil, "a default of the wrong kind fails")
	invalid = Settings{}.behaviorProperties()
	invalid.Fields[2].Min = math.NaN()
	_, err = behaviorBindProperties(invalid, noValues)
	expect(err != nil, "a NaN bound fails")
	invalid = Settings{}.behaviorProperties()
	invalid.Prefix = "TAC_APP_"
	_, err = behaviorBindProperties(invalid, noValues)
	expect(err != nil, "another realm prefix fails")
}

func createConditional(profiles []string, enabled string) (*BehaviorContext, error) {
	options := NewBehaviorOptions("explicit-config", NewBehaviorSignal())
	options.Profiles = profiles
	if enabled != "" {
		options.Properties["YON_FEATURE_ENABLED"] = enabled
	}
	return BehaviorsCreate[ConditionalConfig](options)
}

func checkConditionalConfiguration() {
	inactive := []struct {
		profiles []string
		enabled  string
	}{
		{[]string{}, "true"},
		{[]string{"test"}, ""},
		{[]string{"test"}, "false"},
		{[]string{"production"}, "true"},
	}
	for _, selection := range inactive {
		_, err := createConditional(selection.profiles, selection.enabled)
		expectCode(err, "inactive-configuration", "an inactive selection")
	}
	expect(conditionalConfigureCalls.Load() == 0, "inactive configurations never run")
	for _, profile := range []string{"development", "test"} {
		must(createConditional([]string{profile}, "true")).Dispose()
	}
	expect(conditionalConfigureCalls.Load() == 2, "active configurations run once each")
	manyProfiles := []string{}
	for index := 0; index < 33; index++ {
		manyProfiles = append(manyProfiles, fmt.Sprintf("profile%d", index))
	}
	for _, profiles := range [][]string{{"test", "test"}, {"invalid-profile"}, {"é"}, {strings.Repeat("x", 65)}, manyProfiles} {
		_, err := createConditional(profiles, "true")
		expectCode(err, "configuration", "invalid profiles")
	}
	expect(conditionalConfigureCalls.Load() == 2, "invalid profiles never run the configuration")
}

func configurationContracts() {
	values := propertyValues()
	checkPropertyBinding(values)
	checkRejectedPropertyValues(values)
	checkPropertySizeLimits()
	checkInvalidPropertyPolicies()
	checkConditionalConfiguration()
	checkSharedSetup()
}

type SharedSetup struct{}

var sharedConfigureCalls atomic.Int64

func (SharedSetup) behaviorConditions() BehaviorConfigurationConditions {
	return BehaviorConfigurationConditions{}
}

func (SharedSetup) behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	sharedConfigureCalls.Add(1)
	config := NewBehaviorConfiguration()
	config.Http["notes"] = &Memory{entries: map[string]BehaviorCacheEntry{}}
	return config, nil
}

func createShared(partition string) *BehaviorContext {
	return must(BehaviorsCreate[SharedSetup](NewBehaviorOptions(partition, NewBehaviorSignal())))
}

func checkSharedSetup() {
	writer := createShared("writer")
	reader := createShared("reader")
	expect(sharedConfigureCalls.Load() == 1, "two contexts from one Setup configure it once")
	expect(writer.configuration.Http["notes"] == reader.configuration.Http["notes"], "two contexts share the Setup's adapters")
	writer.Dispose()
	writer.Dispose()
	later := createShared("later")
	expect(sharedConfigureCalls.Load() == 1, "a Setup stays configured while one of its contexts is open")
	reader.Dispose()
	later.Dispose()
	fresh := createShared("fresh")
	expect(sharedConfigureCalls.Load() == 2, "a Setup is configured again after its last context is disposed")
	fresh.Dispose()
}

type ValidatedInput struct{}

func (ValidatedInput) behaviorValidation() BehaviorValidationSchema {
	return BehaviorValidationSchema{Fields: []BehaviorValidationField{
		{Name: "name", NotNull: true, NotBlank: true, HasSize: true, SizeMin: 1, SizeMax: 8},
		{Name: "amount", Min: 1, HasMin: true, Max: 10, HasMax: true},
		{Name: "items", HasSize: true, SizeMin: 1, SizeMax: 2},
	}}
}

func validationFailurePath(value BehaviorValue) string {
	_, err := BehaviorsValidate[ValidatedInput](value)
	expect(err != nil, "validation fails")
	return BehaviorErrorPath(err)
}

func checkValidatedShape(value BehaviorValue) {
	expect(same(must(BehaviorsValidate[ValidatedInput](value)), value), "validation keeps the value")
	expect(validationFailurePath(nil) == "$.object", "null is not an object")
	expect(validationFailurePath(emptyObject()) == "name.notNull", "a missing name fails")
}

func checkBlankNames() {
	blanks := []rune{0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x0020, 0x0085, 0x00a0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000}
	for _, character := range blanks {
		_, err := BehaviorsValidate[ValidatedInput](map[string]any{"name": string(character)})
		expectCode(err, "validation", "a blank name")
		expect(BehaviorErrorPath(err) == "name.notBlank", "a blank name fails notBlank")
	}
	_, err := BehaviorsValidate[ValidatedInput](map[string]any{"name": "\u200b"})
	expect(err == nil, "zero-width space is not Unicode White_Space")
	longName := map[string]any{"name": strings.Repeat("é", 5)}
	expect(validationFailurePath(longName) == "name.size", "the size counts UTF-8 bytes")
}

func checkFieldRules() {
	rules := []struct {
		field string
		value any
		path  string
	}{
		{"name", nil, "name.notNull"},
		{"name", true, "name.notBlank"},
		{"amount", "5", "amount.min"},
		{"amount", 0, "amount.min"},
		{"amount", 11, "amount.max"},
		{"items", true, "items.size"},
		{"items", []any{}, "items.size"},
		{"items", []any{1, 2, 3}, "items.size"},
	}
	for _, rule := range rules {
		data := map[string]any{"name": "name"}
		data[rule.field] = rule.value
		expect(validationFailurePath(data) == rule.path, "rule "+rule.path)
	}
	for _, item := range []any{nil, []any{1}, map[string]any{"one": true, "two": false}} {
		input := map[string]any{"name": "ok", "amount": nil, "items": item}
		_, err := BehaviorsValidate[ValidatedInput](input)
		expect(err == nil, "valid items pass")
	}
}

func checkValidationLimits() {
	_, err := BehaviorsValidate[ValidatedInput](map[string]any{"name": "private-secret-token"})
	expect(!strings.Contains(err.Error(), "private-secret-token"), "errors never echo the value")
	expect(len(BehaviorErrorPath(err)) <= 140, "the path is bounded")
	notANumber := map[string]any{"name": "ok", "amount": math.NaN()}
	expect(validationFailurePath(notANumber) == "$.json", "NaN is not JSON")
	var nested any
	for index := 0; index < 40; index++ {
		nested = []any{nested}
	}
	_, err = BehaviorsValidate[ValidatedInput](map[string]any{"name": "ok", "nested": nested})
	expectCode(err, "validation", "a deep value")
}

func checkInvalidSchemas(value BehaviorValue) {
	schema := ValidatedInput{}.behaviorValidation()
	schema.Fields[0].Name = "input.secret"
	_, err := behaviorValidateValue(schema, value)
	expect(BehaviorErrorPath(err) == "$.schema", "an unsafe field name fails")
	schema = ValidatedInput{}.behaviorValidation()
	schema.Fields[1].Min = math.Inf(1)
	_, err = behaviorValidateValue(schema, value)
	expect(BehaviorErrorPath(err) == "$.schema", "an infinite bound fails")
}

func validationContracts() {
	value := map[string]any{"name": "é", "unknown": true}
	checkValidatedShape(value)
	checkBlankNames()
	checkFieldRules()
	checkValidationLimits()
	checkInvalidSchemas(value)
}

type Owner struct {
	behaviors *BehaviorContext
}

func (owner Owner) behaviorContext() *BehaviorContext {
	return owner.behaviors
}

func (Owner) behaviorMethod(name string) *BehaviorMethodPolicy {
	if name == "list" {
		listed := policy()
		return &listed
	}
	return nil
}

func capacity(context *BehaviorContext, count int) (BehaviorValue, error) {
	return context.Invoke(plainPolicy(), []BehaviorValue{}, func([]BehaviorValue) (BehaviorValue, error) {
		if count == 0 {
			return nil, nil
		}
		return capacity(context, count-1)
	})
}

func checkMissingAuthorization(denied *BehaviorContext) {
	_, err := denied.Invoke(policy(), []BehaviorValue{emptyObject()}, missed)
	expectCode(err, "unauthorized", "a denied read")
	missing := must(BehaviorsCreate[MissingAuthorization](NewBehaviorOptions("missing", NewBehaviorSignal())))
	_, err = missing.Invoke(policy(), []BehaviorValue{emptyObject()}, missed)
	expectCode(err, "invalid", "a read without authorization")
}

func checkEvictionOwners(context *BehaviorContext, denied *BehaviorContext) {
	outsider := Owner{behaviors: denied}
	expect(context.Evict(outsider, "list", []BehaviorValue{emptyObject()}) != nil, "an outsider cannot evict")
	owner := Owner{behaviors: context}
	expect(context.Evict(owner, "remote", []BehaviorValue{emptyObject()}) != nil, "an unknown method cannot evict")
	expect(context.Evict(owner, "list", []BehaviorValue{emptyObject()}) == nil, "the owner evicts")
}

func checkInvalidPageArguments(context *BehaviorContext) {
	for _, page := range []BehaviorValue{
		map[string]any{"page": 1000001},
		map[string]any{"sort": []any{"unknown"}},
		map[string]any{"offset": 1},
	} {
		_, err := context.Invoke(policy(), []BehaviorValue{page}, missed)
		expect(err != nil, "an invalid page fails")
	}
}

func checkValueSizeLimits() {
	_, err := behaviorChecked(strings.Repeat("x", 1048577), 1048576)
	expect(err != nil, "a long text fails")
	longArray := make([]any, 10001)
	_, err = behaviorChecked(longArray, 1048576)
	expect(err != nil, "a long array fails")
	var deep any
	for index := 0; index < 34; index++ {
		deep = []any{deep}
	}
	_, err = behaviorChecked(deep, 1048576)
	expect(err != nil, "a deep value fails")
}

func checkUnsafeHttpRequests() {
	for _, path := range []string{"//evil.test/", "/%2e%2e/escape", "/%252e/escape", "/x?query", "/x\\path", "/%ff"} {
		expect(!behaviorSafePath(path), "unsafe path "+path)
	}
	http := &BehaviorHttpPolicy{Client: "catalog", Method: "GET", Path: "/products"}
	_, err := behaviorHttpRequest(http, map[string]any{"body": emptyObject()})
	expect(err != nil, "a GET body fails")
	_, err = behaviorHttpRequest(http, map[string]any{"headers": map[string]any{"Cookie": "x"}})
	expect(err != nil, "a cookie header fails")
	_, err = behaviorHttpRequest(http, map[string]any{"headers": map[string]any{"X-Token": "x\r\ny"}})
	expect(err != nil, "a line break header fails")
}

func checkRepositoryAndCapacity(context *BehaviorContext) {
	repository := must(BehaviorsRepository[Product](context))
	_, err := repository.Save(emptyObject())
	expect(err != nil, "a save without id fails")
	_, err = repository.Delete(1.5)
	expect(err != nil, "a fractional id fails")
	_, err = capacity(context, 64)
	expectCode(err, "capacity", "nested operations")
	_, err = capacity(context, 0)
	expect(err == nil, "capacity is released after errors")
}

func checkTimeout() {
	options := NewBehaviorOptions("timeout", NewBehaviorSignal())
	options.TimeoutMs = 1
	timed := must(BehaviorsCreate[Config](options))
	_, err := timed.Invoke(plainPolicy(), []BehaviorValue{}, func([]BehaviorValue) (BehaviorValue, error) {
		time.Sleep(10 * time.Millisecond)
		return nil, nil
	})
	expectCode(err, "timeout", "a late method")
}

func negativeContracts(context *BehaviorContext) {
	denied := must(createContext("denied"))
	checkMissingAuthorization(denied)
	checkEvictionOwners(context, denied)
	checkInvalidPageArguments(context)
	checkValueSizeLimits()
	checkUnsafeHttpRequests()
	checkRepositoryAndCapacity(context)
	checkTimeout()
	denied.Dispose()
	_, err := denied.Invoke(plainPolicy(), []BehaviorValue{}, missed)
	expectCode(err, "disposed", "a disposed method")
}

func checkCachedInvocation(context *BehaviorContext) {
	calls := 0
	for attempt := 0; attempt < 2; attempt++ {
		value := must(context.Invoke(policy(), []BehaviorValue{emptyObject()}, func(arguments []BehaviorValue) (BehaviorValue, error) {
			calls++
			expect(arguments[0].(map[string]any)["size"] == 20.0, "the page is normalized")
			return "cached", nil
		}))
		expect(value == "cached", "the cached value is returned")
	}
	expect(calls == 1, "the second call is cached")
}

func checkRepository(context *BehaviorContext) {
	repository := must(BehaviorsRepository[Product](context))
	expect(must(repository.Find(emptyObject())) == "find", "find reaches the adapter")
	expect(must(repository.Save(map[string]any{"id": "one"})) == "save", "save reaches the adapter")
	expect(must(repository.Delete("one")) == "delete", "delete reaches the adapter")
}

func checkBrowserStatus() {
	_, err := behaviorBrowserAnswer(`{"error":{"code":"HTTP_STATUS","status":404}}`)
	expectCode(err, "HTTP_STATUS", "an error status keeps its code")
	expect(BehaviorErrorStatus(err) == 404, "an error status carries its number")
	_, err = behaviorBrowserAnswer(`{"error":{"code":"HTTP_ORIGIN"}}`)
	expect(BehaviorErrorStatus(err) == 0, "other failures carry no status")
}

func checkHttpInvocation(context *BehaviorContext) {
	exchange := plainPolicy()
	exchange.Http = &BehaviorHttpPolicy{Client: "catalog", Method: "GET", Path: "/products"}
	response := must(context.Invoke(exchange, []BehaviorValue{}, func([]BehaviorValue) (BehaviorValue, error) {
		return emptyObject(), nil
	}))
	expect(response == "transport", "the exchange answers")
}

func checkRejectedValues() {
	_, err := behaviorChecked(math.NaN(), 1024)
	expect(err != nil, "NaN fails")
	_, err = behaviorChecked(map[string]any{"constructor": nil}, 1024)
	expect(err != nil, "a reserved key fails")
}

type codecSample struct {
	Name   string `json:"name"`
	Skip   string `json:"-"`
	Count  int
	hidden int
}

type taggedNote struct {
	Text string `json:"text,omitempty"`
}

type taggedStamp struct {
	Seconds int
}

func (stamp *taggedStamp) IsZero() bool {
	return stamp.Seconds <= 0
}

type taggedLevel int

func (level taggedLevel) IsZero() bool {
	return level < 0
}

type taggedRequest struct {
	Title    string            `json:"title,omitempty"`
	Owner    *string           `json:"owner,omitempty"`
	Tags     []string          `json:"tags,omitempty"`
	Labels   map[string]string `json:"labels,omitempty"`
	Count    int               `json:"count,omitempty"`
	Draft    bool              `json:"draft,omitempty"`
	Ratio    float64           `json:",omitempty"`
	Note     taggedNote        `json:"note,omitempty"`
	Any      any               `json:"any,omitempty"`
	Fixed    [0]int            `json:"fixed,omitempty"`
	Hidden   string            `json:"-"`
	Dash     string            `json:"-,"`
	Odd      string            `json:"bad\"name"`
	At       time.Time         `json:"at,omitzero"`
	Stamp    taggedStamp       `json:"stamp,omitzero"`
	Level    taggedLevel       `json:"level,omitzero"`
	Pair     [2]int            `json:"pair,omitzero"`
	Kept     *string           `json:"kept"`
	Quoted   int               `json:"quoted,string"`
	Flag     *bool             `json:"flag,string"`
	Phrase   string            `json:"phrase,string"`
	Skipped  []int             `json:"skipped,string,omitempty"`
	Untagged string
}

func sameAsEncodingJSON(value any, message string) string {
	encoded := must(behaviorEncode(value, 4096))
	reference, failure := json.Marshal(value)
	expect(failure == nil, "encoding/json encodes "+message)
	expect(same(must(behaviorParseJSON(encoded)), must(behaviorParseJSON(string(reference)))), message+" matches encoding/json: "+encoded+" "+string(reference))
	return encoded
}

func checkJSONTags() {
	empty := sameAsEncodingJSON(taggedRequest{Dash: "d"}, "an empty tagged request")
	expect(empty == `{"-":"d","Odd":"","Untagged":"","flag":null,"kept":null,"level":0,"note":{},"phrase":"\"\"","quoted":"0"}`, "omitempty, omitzero, - and string follow encoding/json: "+empty)
	owner := ""
	yes := true
	full := sameAsEncodingJSON(taggedRequest{
		Title: "t", Owner: &owner, Tags: []string{}, Labels: map[string]string{"a": "b"}, Count: 3, Draft: true, Ratio: 0.5,
		Note: taggedNote{Text: "n"}, Any: 0, Hidden: "h", Stamp: taggedStamp{Seconds: 2}, Level: -1,
		Pair: [2]int{0, 1}, Quoted: 12, Flag: &yes, Phrase: "a\"b", Skipped: []int{1},
	}, "a full tagged request")
	expect(strings.Contains(full, `"owner":""`), "a non-nil pointer to an empty string is kept: "+full)
	expect(strings.Contains(full, `"stamp":{"Seconds":2}`), "an IsZero method decides omitzero: "+full)
	expect(!strings.Contains(full, `"level"`) && !strings.Contains(full, `"tags"`), "a zero level and an empty list are left out: "+full)
	expect(strings.Contains(full, `"phrase":"\"a\\\"b\""`) && strings.Contains(full, `"flag":"true"`), "string quotes scalar values: "+full)
	policy := &BehaviorHttpPolicy{Client: "catalog", Method: "POST", Path: "/products"}
	request := must(behaviorHttpRequest(policy, map[string]any{"body": taggedRequest{Dash: "d"}}))
	body := must(behaviorEncode(request.Body, 4096))
	expect(body == empty, "a request body model leaves out its omitempty fields: "+body)
}

func checkJSONCodec() {
	text := must(behaviorEncode(map[string]any{"b": 1, "a": []int{1, 2}, "c": "é\n\u2028<"}, 1024))
	expect(text == `{"a":[1,2],"b":1,"c":"é\n\u2028<"}`, "the codec writes sorted compact JSON: "+text)
	sample := must(behaviorEncode(codecSample{Name: "x", Skip: "y", Count: 2, hidden: 3}, 1024))
	expect(sample == `{"Count":2,"name":"x"}`, "the codec reads json tags: "+sample)
	expect(must(behaviorEncode(json.Number("12.5"), 64)) == "12.5", "a json.Number stays a number")
	expect(must(behaviorEncode(map[int]bool{2: true}, 64)) == `{"2":true}`, "integer keys become text")
	expect(must(behaviorEncode(1e21, 64)) == "1e+21", "large numbers use exponents")
	parsed := must(behaviorParseJSON(` {"smile":"\ud83d\ude00","list":[1,true,null,"a\/b"]} `))
	expect(same(parsed, map[string]any{"smile": "😀", "list": []any{1, true, nil, "a/b"}}), "the codec parses JSON")
	for _, broken := range []string{"", "01", "[1,]", `"\q"`, "{\"a\" 1}", "true false", "\"raw\nline\""} {
		_, err := behaviorParseJSON(broken)
		expect(err != nil, "malformed JSON fails: "+broken)
	}
	_, err := behaviorEncode(func() {}, 64)
	expect(err != nil, "a function is not JSON")
}

type Ledger struct{}

type LedgerAdapter struct {
	log      *[]string
	sequence *int
}

func (ledger LedgerAdapter) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	joined := command.Transaction
	if joined == "" {
		joined = "none"
	}
	*ledger.log = append(*ledger.log, command.Operation+"@"+joined)
	if value, isObject := command.Value.(map[string]any); isObject && value["id"] == "broken" {
		return nil, errors.New("the second write failed")
	}
	return command.Operation, nil
}

func (ledger LedgerAdapter) Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	entry := command.Action
	if command.Transaction != "" {
		entry += ":" + command.Transaction
	}
	*ledger.log = append(*ledger.log, entry)
	if command.Action != "begin" {
		return nil, nil
	}
	*ledger.sequence++
	return fmt.Sprintf("t%d", *ledger.sequence), nil
}

type LedgerLog struct {
	entries  []string
	sequence int
}

var ledgerLog LedgerLog

func (Ledger) behaviorConditions() BehaviorConfigurationConditions {
	return BehaviorConfigurationConditions{}
}

func (Ledger) behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	adapter := LedgerAdapter{log: &ledgerLog.entries, sequence: &ledgerLog.sequence}
	config := NewBehaviorConfiguration()
	config.Entities["ledger"] = adapter
	config.Entities["audit"] = adapter
	config.AuthorizeEntity = func(*BehaviorEntityCommand, *BehaviorExecution) (bool, error) { return true, nil }
	return config, nil
}

type Entry struct{}

func (Entry) behaviorEntity() BehaviorEntityMetadata {
	return BehaviorEntityMetadata{Name: "Entry", Adapter: "ledger", Table: "entries", Id: "id"}
}

type Note struct{}

func (Note) behaviorEntity() BehaviorEntityMetadata {
	return BehaviorEntityMetadata{Name: "Note", Adapter: "audit", Table: "notes", Id: "id"}
}

func settle(service *BehaviorContext, store *BehaviorContext, second string) (BehaviorValue, error) {
	ledger := plainPolicy()
	ledger.Owner = "LedgerService"
	ledger.Name = "settle"
	ledger.Transactional = &BehaviorTransactionPolicy{Adapter: "ledger"}
	return service.Invoke(ledger, []BehaviorValue{}, func([]BehaviorValue) (BehaviorValue, error) {
		entries := must(BehaviorsRepository[Entry](store))
		notes := must(BehaviorsRepository[Note](store))
		if _, err := entries.Save(map[string]any{"id": "first"}); err != nil {
			return nil, err
		}
		if _, err := notes.Save(map[string]any{"id": "note"}); err != nil {
			return nil, err
		}
		if _, err := entries.Save(map[string]any{"id": second}); err != nil {
			return nil, err
		}
		return "settled", nil
	})
}

func checkJoinedTransaction() {
	service := must(BehaviorsCreate[Ledger](NewBehaviorOptions("ledger", NewBehaviorSignal())))
	store := must(BehaviorsCreate[Ledger](NewBehaviorOptions("entries", NewBehaviorSignal())))
	expect(must(settle(service, store, "second")) == "settled", "a joined transaction answers its body")
	expect(same(ledgerLog.entries, []string{"begin", "save@t1", "save@none", "save@t1", "commit:t1"}), fmt.Sprint("a store in another context joins the open transaction: ", ledgerLog.entries))
	ledgerLog.entries = nil
	_, err := settle(service, store, "broken")
	expect(err != nil, "a failed second write fails the transaction")
	expect(same(ledgerLog.entries, []string{"begin", "save@t2", "save@none", "save@t2", "rollback:t2"}), fmt.Sprint("a failed second write rolls back the first: ", ledgerLog.entries))
}

func main() {
	cancellationRetryContracts()
	checkJSONCodec()
	checkJSONTags()
	validationContracts()
	configurationContracts()
	mutationContracts()
	queryContracts()
	signal := NewBehaviorSignal()
	context := must(BehaviorsCreate[Config](NewBehaviorOptions("tenant-a", signal)))
	checkCachedInvocation(context)
	checkRepository(context)
	checkJoinedTransaction()
	checkHttpInvocation(context)
	checkBrowserStatus()
	checkRejectedValues()
	negativeContracts(context)
	signal.Abort()
	_, err := context.Invoke(policy(), []BehaviorValue{}, func([]BehaviorValue) (BehaviorValue, error) {
		return nil, nil
	})
	expectCode(err, "aborted", "an aborted context")
	context.Dispose()
	fmt.Println("go behavior runtime contracts passed")
}
