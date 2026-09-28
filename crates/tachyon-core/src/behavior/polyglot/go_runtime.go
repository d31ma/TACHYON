package main

import (
	"errors"
	"math"
	"reflect"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"time"
	"unicode/utf16"
	"unicode/utf8"
)

type BehaviorValue = any

type BehaviorError struct {
	Code    string
	Message string
	Path    string
}

func (failure *BehaviorError) Error() string {
	text := failure.Code + ": " + failure.Message
	if failure.Path != "" {
		text += " (" + failure.Path + ")"
	}
	return text
}

func BehaviorErrorCode(err error) string {
	var failure *BehaviorError
	if errors.As(err, &failure) {
		return failure.Code
	}
	return ""
}

func BehaviorErrorPath(err error) string {
	var failure *BehaviorError
	if errors.As(err, &failure) {
		return failure.Path
	}
	return ""
}

func behaviorErrorMessage(err error) string {
	var failure *BehaviorError
	if errors.As(err, &failure) {
		return failure.Message
	}
	return err.Error()
}

func behaviorError(code string, message string) error {
	return &BehaviorError{Code: code, Message: message}
}

func behaviorInvalid() error {
	return behaviorError("invalid", "Invalid or out-of-bounds behavior data.")
}

func behaviorRetryable(err error) bool {
	switch BehaviorErrorCode(err) {
	case "invalid", "unauthorized", "capacity", "cancelled", "timeout", "configuration":
		return false
	}
	return true
}

func behaviorBackoff(retry *BehaviorRetryPolicy, attempt int) time.Duration {
	delay := math.Min(float64(retry.DelayMs)*math.Pow(retry.Multiplier, float64(attempt)), float64(retry.MaxDelayMs))
	jitter := 0.0
	if retry.JitterMs > 0 {
		jitter = float64(behaviorNow() % (retry.JitterMs + 1))
	}
	milliseconds := math.Max(math.Round(delay+jitter), 0)
	return time.Duration(milliseconds) * time.Millisecond
}

func behaviorEncode(value BehaviorValue, limit int) (string, error) {
	if limit <= 0 || limit > 1048576 {
		return "", behaviorInvalid()
	}
	normalized, err := behaviorNormalize(reflect.ValueOf(value), 0)
	if err != nil {
		return "", err
	}
	nodes := 0
	if !behaviorWithinBounds(normalized, 0, &nodes) {
		return "", behaviorInvalid()
	}
	var builder strings.Builder
	behaviorWriteJSON(&builder, normalized)
	if builder.Len() > limit {
		return "", behaviorInvalid()
	}
	return builder.String(), nil
}

func behaviorChecked(value BehaviorValue, limit int) (BehaviorValue, error) {
	if _, err := behaviorEncode(value, limit); err != nil {
		return nil, err
	}
	return behaviorNormalize(reflect.ValueOf(value), 0)
}

func behaviorNormalize(value reflect.Value, depth int) (any, error) {
	if depth > 64 {
		return nil, behaviorInvalid()
	}
	if !value.IsValid() {
		return nil, nil
	}
	if value.Type().Name() == "Number" && value.Type().PkgPath() == "encoding/json" {
		number, err := strconv.ParseFloat(value.String(), 64)
		if err != nil {
			return nil, behaviorInvalid()
		}
		return number, nil
	}
	switch value.Kind() {
	case reflect.Interface, reflect.Pointer:
		if value.IsNil() {
			return nil, nil
		}
		return behaviorNormalize(value.Elem(), depth+1)
	case reflect.Bool:
		return value.Bool(), nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return float64(value.Int()), nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return float64(value.Uint()), nil
	case reflect.Float32, reflect.Float64:
		number := value.Float()
		if math.IsNaN(number) || math.IsInf(number, 0) {
			return nil, behaviorInvalid()
		}
		return number, nil
	case reflect.String:
		return value.String(), nil
	case reflect.Slice:
		if value.IsNil() {
			return nil, nil
		}
		return behaviorNormalizeList(value, depth)
	case reflect.Array:
		return behaviorNormalizeList(value, depth)
	case reflect.Map:
		if value.IsNil() {
			return nil, nil
		}
		return behaviorNormalizeMap(value, depth)
	case reflect.Struct:
		return behaviorNormalizeStruct(value, depth)
	}
	return nil, behaviorInvalid()
}

func behaviorNormalizeList(value reflect.Value, depth int) (any, error) {
	items := make([]any, 0, value.Len())
	for index := 0; index < value.Len(); index++ {
		item, err := behaviorNormalize(value.Index(index), depth+1)
		if err != nil {
			return nil, err
		}
		items = append(items, item)
	}
	return items, nil
}

func behaviorNormalizeMap(value reflect.Value, depth int) (any, error) {
	object := map[string]any{}
	iterator := value.MapRange()
	for iterator.Next() {
		key := iterator.Key()
		name := ""
		switch key.Kind() {
		case reflect.String:
			name = key.String()
		case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
			name = strconv.FormatInt(key.Int(), 10)
		case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
			name = strconv.FormatUint(key.Uint(), 10)
		default:
			return nil, behaviorInvalid()
		}
		item, err := behaviorNormalize(iterator.Value(), depth+1)
		if err != nil {
			return nil, err
		}
		object[name] = item
	}
	return object, nil
}

func behaviorNormalizeStruct(value reflect.Value, depth int) (any, error) {
	object := map[string]any{}
	for index := 0; index < value.NumField(); index++ {
		field := value.Type().Field(index)
		if !field.IsExported() {
			continue
		}
		name := field.Name
		if tag, tagged := field.Tag.Lookup("json"); tagged {
			tagName := strings.Split(tag, ",")[0]
			if tagName == "-" {
				continue
			}
			if tagName != "" {
				name = tagName
			}
		}
		item, err := behaviorNormalize(value.Field(index), depth+1)
		if err != nil {
			return nil, err
		}
		object[name] = item
	}
	return object, nil
}

func behaviorWriteJSON(builder *strings.Builder, value any) {
	switch typed := value.(type) {
	case nil:
		builder.WriteString("null")
	case bool:
		builder.WriteString(strconv.FormatBool(typed))
	case float64:
		builder.WriteString(behaviorNumberText(typed))
	case string:
		behaviorWriteString(builder, typed)
	case []any:
		builder.WriteByte('[')
		for index, item := range typed {
			if index > 0 {
				builder.WriteByte(',')
			}
			behaviorWriteJSON(builder, item)
		}
		builder.WriteByte(']')
	case map[string]any:
		builder.WriteByte('{')
		for index, key := range behaviorSortedKeys(typed) {
			if index > 0 {
				builder.WriteByte(',')
			}
			behaviorWriteString(builder, key)
			builder.WriteByte(':')
			behaviorWriteJSON(builder, typed[key])
		}
		builder.WriteByte('}')
	}
}

func behaviorNumberText(number float64) string {
	absolute := math.Abs(number)
	if absolute != 0 && (absolute < 1e-6 || absolute >= 1e21) {
		text := strconv.FormatFloat(number, 'e', -1, 64)
		text = strings.Replace(text, "e-0", "e-", 1)
		return strings.Replace(text, "e+0", "e+", 1)
	}
	return strconv.FormatFloat(number, 'f', -1, 64)
}

func behaviorWriteString(builder *strings.Builder, text string) {
	builder.WriteByte('"')
	for _, character := range strings.ToValidUTF8(text, "\uFFFD") {
		switch character {
		case '"':
			builder.WriteString(`\"`)
		case '\\':
			builder.WriteString(`\\`)
		case '\n':
			builder.WriteString(`\n`)
		case '\r':
			builder.WriteString(`\r`)
		case '\t':
			builder.WriteString(`\t`)
		case '\u2028', '\u2029':
			builder.WriteString(`\u` + strconv.FormatInt(int64(character), 16))
		default:
			if character < 0x20 {
				builder.WriteString(`\u00`)
				builder.WriteString(strconv.FormatInt(int64(character)>>4, 16))
				builder.WriteString(strconv.FormatInt(int64(character)&15, 16))
			} else {
				builder.WriteRune(character)
			}
		}
	}
	builder.WriteByte('"')
}

type behaviorJSONReader struct {
	text  string
	at    int
	depth int
}

func behaviorParseJSON(text string) (any, error) {
	reader := &behaviorJSONReader{text: text}
	value, err := reader.value()
	if err != nil {
		return nil, err
	}
	reader.skipSpace()
	if reader.at != len(reader.text) {
		return nil, behaviorInvalid()
	}
	return value, nil
}

func (reader *behaviorJSONReader) skipSpace() {
	for reader.at < len(reader.text) && strings.IndexByte(" \t\r\n", reader.text[reader.at]) >= 0 {
		reader.at++
	}
}

func (reader *behaviorJSONReader) value() (any, error) {
	reader.skipSpace()
	if reader.at >= len(reader.text) {
		return nil, behaviorInvalid()
	}
	switch reader.text[reader.at] {
	case '{':
		return reader.object()
	case '[':
		return reader.list()
	case '"':
		return reader.quoted()
	case 't':
		return reader.word("true", true)
	case 'f':
		return reader.word("false", false)
	case 'n':
		return reader.word("null", nil)
	}
	return reader.number()
}

func (reader *behaviorJSONReader) word(word string, value any) (any, error) {
	if !strings.HasPrefix(reader.text[reader.at:], word) {
		return nil, behaviorInvalid()
	}
	reader.at += len(word)
	return value, nil
}

func (reader *behaviorJSONReader) number() (any, error) {
	start := reader.at
	for reader.at < len(reader.text) && strings.IndexByte("+-.eE0123456789", reader.text[reader.at]) >= 0 {
		reader.at++
	}
	text := reader.text[start:reader.at]
	if !behaviorNumberToken(text, false) {
		return nil, behaviorInvalid()
	}
	number, err := strconv.ParseFloat(text, 64)
	if err != nil {
		return nil, behaviorInvalid()
	}
	return number, nil
}

func (reader *behaviorJSONReader) enter() error {
	reader.depth++
	if reader.depth > 64 {
		return behaviorInvalid()
	}
	reader.at++
	reader.skipSpace()
	return nil
}

func (reader *behaviorJSONReader) list() (any, error) {
	if err := reader.enter(); err != nil {
		return nil, err
	}
	items := []any{}
	if reader.next(']') {
		reader.depth--
		return items, nil
	}
	for {
		item, err := reader.value()
		if err != nil {
			return nil, err
		}
		items = append(items, item)
		reader.skipSpace()
		if reader.next(']') {
			reader.depth--
			return items, nil
		}
		if !reader.next(',') {
			return nil, behaviorInvalid()
		}
	}
}

func (reader *behaviorJSONReader) object() (any, error) {
	if err := reader.enter(); err != nil {
		return nil, err
	}
	object := map[string]any{}
	if reader.next('}') {
		reader.depth--
		return object, nil
	}
	for {
		reader.skipSpace()
		if reader.at >= len(reader.text) || reader.text[reader.at] != '"' {
			return nil, behaviorInvalid()
		}
		key, err := reader.quoted()
		if err != nil {
			return nil, err
		}
		reader.skipSpace()
		if !reader.next(':') {
			return nil, behaviorInvalid()
		}
		item, err := reader.value()
		if err != nil {
			return nil, err
		}
		object[key.(string)] = item
		reader.skipSpace()
		if reader.next('}') {
			reader.depth--
			return object, nil
		}
		if !reader.next(',') {
			return nil, behaviorInvalid()
		}
	}
}

func (reader *behaviorJSONReader) next(character byte) bool {
	if reader.at < len(reader.text) && reader.text[reader.at] == character {
		reader.at++
		return true
	}
	return false
}

func (reader *behaviorJSONReader) quoted() (any, error) {
	reader.at++
	var builder strings.Builder
	for reader.at < len(reader.text) {
		character := reader.text[reader.at]
		reader.at++
		switch {
		case character == '"':
			return builder.String(), nil
		case character == '\\':
			if err := reader.escape(&builder); err != nil {
				return nil, err
			}
		case character < 0x20:
			return nil, behaviorInvalid()
		default:
			builder.WriteByte(character)
		}
	}
	return nil, behaviorInvalid()
}

func (reader *behaviorJSONReader) escape(builder *strings.Builder) error {
	if reader.at >= len(reader.text) {
		return behaviorInvalid()
	}
	character := reader.text[reader.at]
	reader.at++
	replacements := map[byte]string{'"': `"`, '\\': `\`, '/': "/", 'b': "\b", 'f': "\f", 'n': "\n", 'r': "\r", 't': "\t"}
	if replacement, known := replacements[character]; known {
		builder.WriteString(replacement)
		return nil
	}
	if character != 'u' {
		return behaviorInvalid()
	}
	first, err := reader.hexadecimal()
	if err != nil {
		return err
	}
	if first >= 0xd800 && first < 0xdc00 && strings.HasPrefix(reader.text[reader.at:], `\u`) {
		reader.at += 2
		second, err := reader.hexadecimal()
		if err != nil {
			return err
		}
		builder.WriteRune(utf16.DecodeRune(first, second))
		return nil
	}
	builder.WriteRune(first)
	return nil
}

func (reader *behaviorJSONReader) hexadecimal() (rune, error) {
	if reader.at+4 > len(reader.text) {
		return 0, behaviorInvalid()
	}
	value, err := strconv.ParseUint(reader.text[reader.at:reader.at+4], 16, 32)
	if err != nil {
		return 0, behaviorInvalid()
	}
	reader.at += 4
	return rune(value), nil
}

func behaviorWithinBounds(value any, depth int, nodes *int) bool {
	*nodes++
	if *nodes > 10000 || depth > 32 {
		return false
	}
	switch typed := value.(type) {
	case []any:
		if len(typed) > 10000 {
			return false
		}
		for _, item := range typed {
			if !behaviorWithinBounds(item, depth+1, nodes) {
				return false
			}
		}
	case map[string]any:
		if len(typed) > 10000 {
			return false
		}
		for key, item := range typed {
			if behaviorReservedKey(key) || (key == "tac" && depth != 0) {
				return false
			}
			if !behaviorWithinBounds(item, depth+1, nodes) {
				return false
			}
		}
	}
	return true
}

func behaviorReservedKey(key string) bool {
	switch key {
	case "__proto__", "prototype", "constructor", "behaviors":
		return true
	}
	return false
}

type BehaviorSignal struct {
	aborted atomic.Bool
}

func NewBehaviorSignal() *BehaviorSignal {
	return &BehaviorSignal{}
}

func (signal *BehaviorSignal) Abort() {
	signal.aborted.Store(true)
}

func (signal *BehaviorSignal) Aborted() bool {
	return signal != nil && signal.aborted.Load()
}

type BehaviorExecution struct {
	Environment string
	Partition   string
	Deadline    int64
	owner       *BehaviorSignal
	disposed    *BehaviorSignal
	until       time.Time
}

func (execution *BehaviorExecution) Check() error {
	if execution.disposed.Aborted() {
		return behaviorError("disposed", "Behavior context is disposed.")
	}
	if execution.owner.Aborted() {
		return behaviorError("aborted", "Behavior operation was aborted.")
	}
	if !time.Now().Before(execution.until) {
		return behaviorError("timeout", "Behavior operation timed out.")
	}
	return nil
}

func (execution *BehaviorExecution) Aborted() bool {
	return execution.owner.Aborted() || execution.disposed.Aborted() || !time.Now().Before(execution.until)
}

type BehaviorOptions struct {
	Partition  string
	Signal     *BehaviorSignal
	TimeoutMs  int64
	Profiles   []string
	Properties map[string]string
}

func NewBehaviorOptions(partition string, signal *BehaviorSignal) *BehaviorOptions {
	return &BehaviorOptions{
		Partition:  partition,
		Signal:     signal,
		TimeoutMs:  10000,
		Profiles:   []string{},
		Properties: map[string]string{},
	}
}

func (options *BehaviorOptions) WithApplication(application BehaviorValue) (*BehaviorOptions, error) {
	properties, err := behaviorApplicationProperties(application)
	if err != nil {
		return nil, err
	}
	options.Properties = properties
	return options, nil
}

type BehaviorCachePolicy struct {
	Name       string
	TtlMs      int64
	MaxBytes   int
	MaxEntries int
}

type BehaviorPagePolicy struct {
	Argument    int
	DefaultSize int64
	MaxSize     int64
	Sort        []string
}

type BehaviorArgumentName struct {
	Name     string
	Argument int
}

type BehaviorHttpPolicy struct {
	Client        string
	Method        string
	Path          string
	Query         []BehaviorArgumentName
	Headers       []BehaviorArgumentName
	Body          int
	HasBody       bool
	PathVariables []BehaviorArgumentName
}

type BehaviorCacheMutationPolicy struct {
	Operation string
	Method    string
	Arguments []int
	KeyArgs   []int
	Pageable  *BehaviorPagePolicy
}

type BehaviorRetryPolicy struct {
	MaxRetries int
	DelayMs    int64
	Multiplier float64
	MaxDelayMs int64
	JitterMs   int64
}

type BehaviorLimitPolicy struct {
	Limit    int
	Blocking bool
}

type BehaviorTransactionPolicy struct {
	Adapter string
}

type BehaviorMethodPolicy struct {
	Owner         string
	Name          string
	Cache         *BehaviorCachePolicy
	KeyArgs       []int
	Pageable      *BehaviorPagePolicy
	Http          *BehaviorHttpPolicy
	Mutation      *BehaviorCacheMutationPolicy
	Retry         *BehaviorRetryPolicy
	Limit         *BehaviorLimitPolicy
	Transactional *BehaviorTransactionPolicy
}

type BehaviorCacheEntry struct {
	Value     BehaviorValue
	ExpiresAt int64
}

type BehaviorCacheWrite struct {
	Execution *BehaviorExecution
	Policy    BehaviorCachePolicy
	ExpiresAt int64
}

type BehaviorCacheAdapter interface {
	Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error)
	Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error
	Delete(key string, execution *BehaviorExecution) error
}

type BehaviorAuthorizationCall struct {
	Operation    string
	Cache        string
	Owner        string
	Method       string
	KeyArguments []BehaviorValue
}

type BehaviorHttpRequest struct {
	Method           string
	Path             string
	Query            map[string]any
	Headers          map[string]any
	Body             BehaviorValue
	HasBody          bool
	Redirect         string
	Credentials      string
	MaxResponseBytes int
}

type BehaviorHttpAdapter interface {
	Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error)
}

type BehaviorEntityMetadata struct {
	Name    string
	Adapter string
	Table   string
	Id      string
}

type BehaviorQueryPredicate struct {
	Field       string
	Operator    string
	Argument    int
	HasArgument bool
}

type BehaviorQuerySort struct {
	Field     string
	Direction string
}

type BehaviorQueryPolicy struct {
	Entity          BehaviorEntityMetadata
	Fields          []string
	DefaultSize     int64
	MaxSize         int64
	Operation       string
	Predicates      []BehaviorQueryPredicate
	OrderBy         []BehaviorQuerySort
	PageArgument    int
	HasPageArgument bool
}

type BehaviorEntityCommand struct {
	Entity      string
	Table       string
	IdField     string
	Operation   string
	Value       BehaviorValue
	Page        BehaviorValue
	Id          BehaviorValue
	Query       BehaviorValue
	Transaction string
}

type BehaviorTransactionCommand struct {
	Action      string
	Transaction string
}

type BehaviorEntityAdapter interface {
	Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error)
}

type BehaviorTransactionAdapter interface {
	Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error)
}

type BehaviorConfiguration struct {
	Caches          map[string]BehaviorCacheAdapter
	Http            map[string]BehaviorHttpAdapter
	Entities        map[string]BehaviorEntityAdapter
	Authorize       func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error)
	AuthorizeEntity func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error)
}

func NewBehaviorConfiguration() *BehaviorConfiguration {
	return &BehaviorConfiguration{
		Caches:   map[string]BehaviorCacheAdapter{},
		Http:     map[string]BehaviorHttpAdapter{},
		Entities: map[string]BehaviorEntityAdapter{},
	}
}

type BehaviorConfigurationConditions struct {
	Profiles []string
	Property *BehaviorPropertyCondition
}

type BehaviorPropertyCondition struct {
	Name           string
	HavingValue    string
	MatchIfMissing bool
}

type BehaviorPropertyField struct {
	Field   string
	Name    string
	Kind    string
	Default BehaviorValue
	Min     float64
	HasMin  bool
	Max     float64
	HasMax  bool
}

type BehaviorPropertiesPolicy struct {
	Prefix string
	Fields []BehaviorPropertyField
}

type BehaviorValuesPolicy struct {
	Fields []BehaviorPropertyField
}

type BehaviorValidationField struct {
	Name     string
	NotNull  bool
	NotBlank bool
	HasSize  bool
	SizeMin  int
	SizeMax  int
	Min      float64
	HasMin   bool
	Max      float64
	HasMax   bool
}

type BehaviorValidationSchema struct {
	Fields []BehaviorValidationField
}

type BehaviorMappingField struct {
	Target   string
	Source   []string
	Constant BehaviorValue
}

type behaviorConfigurationSource interface {
	behaviorConfigure(execution *BehaviorExecution) (*BehaviorConfiguration, error)
	behaviorConditions() BehaviorConfigurationConditions
}

type behaviorPropertiesSource interface {
	behaviorProperties() BehaviorPropertiesPolicy
}

type behaviorValuesSource interface {
	behaviorValues() BehaviorValuesPolicy
}

type behaviorValidated interface {
	behaviorValidation() BehaviorValidationSchema
}

type behaviorEntitySource interface {
	behaviorEntity() BehaviorEntityMetadata
}

type behaviorService interface {
	behaviorContext() *BehaviorContext
	behaviorMethod(name string) *BehaviorMethodPolicy
}

func BehaviorsValidate[V behaviorValidated](value BehaviorValue) (BehaviorValue, error) {
	var validated V
	return behaviorValidateValue(validated.behaviorValidation(), value)
}

func BehaviorsProperties[P behaviorPropertiesSource](input any) (BehaviorValue, error) {
	var source P
	values, err := behaviorPropertyInputs(input)
	if err == nil {
		var bound BehaviorValue
		bound, err = behaviorBindProperties(source.behaviorProperties(), values)
		if err == nil {
			return bound, nil
		}
	}
	return nil, behaviorError("configuration-properties", "Invalid or missing explicit configuration properties.")
}

func BehaviorsValues[V behaviorValuesSource](application BehaviorValue) (BehaviorValue, error) {
	var source V
	bound, err := behaviorBindValues(source.behaviorValues(), application)
	if err != nil {
		return nil, behaviorError("configuration-properties", "Invalid or missing application configuration value.")
	}
	return bound, nil
}

func BehaviorsCreate[C behaviorConfigurationSource](options *BehaviorOptions) (*BehaviorContext, error) {
	var source C
	partition := strings.TrimSpace(options.Partition)
	if partition == "" || len(options.Partition) > 256 || options.TimeoutMs < 1 || options.TimeoutMs > 60000 {
		return nil, behaviorInvalid()
	}
	if behaviorValidateProperties(options.Properties) != nil || behaviorValidateProfiles(options.Profiles) != nil {
		return nil, behaviorError("configuration", "Invalid explicit configuration inputs.")
	}
	disposed := NewBehaviorSignal()
	execution := behaviorNewExecution(options, disposed)
	if err := execution.Check(); err != nil {
		return nil, err
	}
	if err := behaviorActiveConfiguration(source.behaviorConditions(), options); err != nil {
		return nil, err
	}
	configuration, err := behaviorLoadConfiguration(source, execution)
	if err != nil {
		disposed.Abort()
		return nil, err
	}
	copied := *options
	return &BehaviorContext{
		options:       &copied,
		configuration: configuration,
		disposed:      disposed,
		slots:         map[string]int{},
	}, nil
}

func BehaviorsRepository[E behaviorEntitySource](context *BehaviorContext) (*BehaviorRepository, error) {
	var source E
	metadata := source.behaviorEntity()
	for _, name := range []string{metadata.Name, metadata.Adapter, metadata.Table, metadata.Id} {
		if !behaviorIdentifier(name) {
			return nil, behaviorInvalid()
		}
	}
	return &BehaviorRepository{context: context, metadata: metadata}, nil
}

func BehaviorsGatewayArguments(arguments []BehaviorValue) (string, error) {
	if arguments == nil {
		arguments = []BehaviorValue{}
	}
	text, err := behaviorEncode(arguments, 1048576)
	if err != nil {
		return "", behaviorGatewayFailure("invalid")
	}
	return text, nil
}

func BehaviorsGatewayResult(answer string) (BehaviorValue, error) {
	parsed, err := behaviorParseJSON(answer)
	fields, isObject := parsed.(map[string]any)
	if err != nil || !isObject {
		return nil, behaviorGatewayFailure("adapter")
	}
	if result, present := fields["result"]; present {
		return result, nil
	}
	kind, isText := fields["error"].(string)
	if !isText {
		return nil, behaviorGatewayFailure("adapter")
	}
	return nil, behaviorGatewayFailure(kind)
}

func behaviorGatewayFailure(kind string) error {
	switch kind {
	case "invalid", "timeout", "capacity":
		return behaviorError(kind, "Gateway call failed.")
	}
	return behaviorError("adapter", "Gateway call failed.")
}

func BehaviorMapValue(value BehaviorValue, plan []BehaviorMappingField) (BehaviorValue, error) {
	object, isObject := value.(map[string]any)
	if !isObject {
		return nil, behaviorInvalid()
	}
	output := map[string]any{}
	for _, field := range plan {
		if field.Source == nil {
			output[field.Target] = field.Constant
			continue
		}
		var current any = object
		for _, name := range field.Source {
			nested, isNested := current.(map[string]any)
			if !isNested {
				current = nil
				break
			}
			current = nested[name]
		}
		output[field.Target] = current
	}
	return behaviorChecked(output, 1048576)
}

func behaviorValidationError(field string, rule string) error {
	return &BehaviorError{Code: "validation", Message: "Behavior validation failed.", Path: field + "." + rule}
}

func behaviorUnicodeWhitespace(character rune) bool {
	switch {
	case character >= 0x0009 && character <= 0x000d:
		return true
	case character >= 0x2000 && character <= 0x200a:
		return true
	}
	switch character {
	case 0x0020, 0x0085, 0x00a0, 0x1680, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000:
		return true
	}
	return false
}

func behaviorValidateValue(schema BehaviorValidationSchema, value BehaviorValue) (BehaviorValue, error) {
	checked, err := behaviorChecked(value, 1048576)
	if err != nil {
		return nil, behaviorValidationError("$", "json")
	}
	values, isObject := checked.(map[string]any)
	if !isObject {
		return nil, behaviorValidationError("$", "object")
	}
	if len(schema.Fields) > 64 {
		return nil, behaviorValidationError("$", "schema")
	}
	seen := map[string]bool{}
	for _, field := range schema.Fields {
		if !behaviorIdentifier(field.Name) || seen[field.Name] || behaviorInvalidValidationBounds(field) {
			return nil, behaviorValidationError("$", "schema")
		}
		seen[field.Name] = true
		if err := behaviorValidateField(field, values[field.Name]); err != nil {
			return nil, err
		}
	}
	return checked, nil
}

func behaviorInvalidValidationBounds(field BehaviorValidationField) bool {
	if field.HasSize && (field.SizeMin > field.SizeMax || field.SizeMax > 1048576 || field.SizeMin < 0) {
		return true
	}
	if field.HasMin && !behaviorPropertyNumber(field.Min, false) {
		return true
	}
	if field.HasMax && !behaviorPropertyNumber(field.Max, false) {
		return true
	}
	return field.HasMin && field.HasMax && field.Min > field.Max
}

func behaviorValidateField(field BehaviorValidationField, item BehaviorValue) error {
	absent := item == nil
	if field.NotNull && absent {
		return behaviorValidationError(field.Name, "notNull")
	}
	if field.NotBlank && !behaviorNotBlank(item) {
		return behaviorValidationError(field.Name, "notBlank")
	}
	if absent {
		return nil
	}
	if field.HasSize {
		size := -1
		switch typed := item.(type) {
		case string:
			size = len(typed)
		case []any:
			size = len(typed)
		case map[string]any:
			size = len(typed)
		}
		if size < field.SizeMin || size > field.SizeMax {
			return behaviorValidationError(field.Name, "size")
		}
	}
	number, isNumber := item.(float64)
	if field.HasMin && (!isNumber || number < field.Min) {
		return behaviorValidationError(field.Name, "min")
	}
	if field.HasMax && (!isNumber || number > field.Max) {
		return behaviorValidationError(field.Name, "max")
	}
	return nil
}

func behaviorNotBlank(item BehaviorValue) bool {
	text, isText := item.(string)
	if !isText || text == "" {
		return false
	}
	for _, character := range text {
		if !behaviorUnicodeWhitespace(character) {
			return true
		}
	}
	return false
}

func behaviorFrameworkPrefix() string {
	if behaviorEnvironment == "tac" {
		return "TAC_"
	}
	return "YON_"
}

func behaviorEnvironmentName(name string) bool {
	if name == "" || len(name) > 256 || name[0] < 'A' || name[0] > 'Z' {
		return false
	}
	for index := 0; index < len(name); index++ {
		current := name[index]
		if !(current >= 'A' && current <= 'Z') && !(current >= '0' && current <= '9') && current != '_' {
			return false
		}
	}
	return true
}

func behaviorValidateProperties(values map[string]string) error {
	if len(values) > 256 {
		return behaviorInvalid()
	}
	object := map[string]any{}
	for name, value := range values {
		if !behaviorEnvironmentName(name) || !strings.HasPrefix(name, behaviorFrameworkPrefix()) || len(value) > 4096 {
			return behaviorInvalid()
		}
		object[name] = value
	}
	_, err := behaviorEncode(object, 65536)
	return err
}

func behaviorValidateProfiles(profiles []string) error {
	unique := map[string]bool{}
	for _, profile := range profiles {
		if profile == "" || len(profile) > 64 || unique[profile] || !behaviorProfileName(profile) {
			return behaviorInvalid()
		}
		unique[profile] = true
		if len(unique) > 32 {
			return behaviorInvalid()
		}
	}
	return nil
}

func behaviorProfileName(profile string) bool {
	for index := 0; index < len(profile); index++ {
		current := profile[index]
		letter := (current >= 'a' && current <= 'z') || (current >= 'A' && current <= 'Z')
		digit := index > 0 && current >= '0' && current <= '9'
		if !letter && !digit && current != '_' && current != '$' {
			return false
		}
	}
	return true
}

func behaviorActiveConfiguration(conditions BehaviorConfigurationConditions, options *BehaviorOptions) error {
	if behaviorValidateProfiles(conditions.Profiles) != nil {
		return behaviorError("configuration", "Invalid explicit configuration conditions.")
	}
	profilesMatch := len(conditions.Profiles) == 0
	for _, profile := range conditions.Profiles {
		for _, active := range options.Profiles {
			if active == profile {
				profilesMatch = true
			}
		}
	}
	propertyMatch := true
	if conditions.Property != nil {
		matches, err := behaviorPropertyConditionMatches(conditions.Property, options)
		if err != nil {
			return err
		}
		propertyMatch = matches
	}
	if !profilesMatch || !propertyMatch {
		return behaviorError("inactive-configuration", "Setup is inactive for the explicit selection.")
	}
	return nil
}

func behaviorPropertyConditionMatches(condition *BehaviorPropertyCondition, options *BehaviorOptions) (bool, error) {
	if !behaviorEnvironmentName(condition.Name) || !strings.HasPrefix(condition.Name, behaviorFrameworkPrefix()) || len(condition.HavingValue) > 4096 {
		return false, behaviorError("configuration", "Invalid explicit configuration conditions.")
	}
	nestedPrefix := condition.Name + "_"
	for key := range options.Properties {
		if strings.HasPrefix(key, nestedPrefix) {
			return false, nil
		}
	}
	value, present := options.Properties[condition.Name]
	if !present {
		return condition.MatchIfMissing, nil
	}
	return value == condition.HavingValue, nil
}

func behaviorPropertyNumber(value float64, integer bool) bool {
	if math.IsNaN(value) || math.IsInf(value, 0) || math.Abs(value) > 9007199254740991 {
		return false
	}
	return !integer || value == math.Trunc(value)
}

func behaviorNumberToken(value string, integer bool) bool {
	position := 0
	if strings.HasPrefix(value, "-") {
		position = 1
	}
	if position >= len(value) {
		return false
	}
	switch {
	case value[position] == '0':
		position++
	case value[position] >= '1' && value[position] <= '9':
		position = behaviorSkipDigits(value, position+1)
	default:
		return false
	}
	if !integer && position < len(value) && value[position] == '.' {
		first := position + 1
		position = behaviorSkipDigits(value, first)
		if first == position {
			return false
		}
	}
	if !integer && position < len(value) && (value[position] == 'e' || value[position] == 'E') {
		position++
		if position < len(value) && (value[position] == '+' || value[position] == '-') {
			position++
		}
		first := position
		position = behaviorSkipDigits(value, first)
		if first == position {
			return false
		}
	}
	return position == len(value)
}

func behaviorSkipDigits(value string, position int) int {
	for position < len(value) && value[position] >= '0' && value[position] <= '9' {
		position++
	}
	return position
}

func behaviorPropertyValue(field BehaviorPropertyField, raw string, present bool) (BehaviorValue, error) {
	numeric := field.Kind == "integer" || field.Kind == "number"
	integer := field.Kind == "integer"
	switch field.Kind {
	case "string", "boolean", "integer", "number":
	default:
		return nil, behaviorInvalid()
	}
	if field.HasMin && (!numeric || !behaviorPropertyNumber(field.Min, integer)) {
		return nil, behaviorInvalid()
	}
	if field.HasMax && (!numeric || !behaviorPropertyNumber(field.Max, integer)) {
		return nil, behaviorInvalid()
	}
	if field.HasMin && field.HasMax && field.Min > field.Max {
		return nil, behaviorInvalid()
	}
	var value BehaviorValue
	if present {
		parsed, err := behaviorParseProperty(field.Kind, raw, integer)
		if err != nil {
			return nil, err
		}
		value = parsed
	} else {
		if field.Default == nil {
			return nil, behaviorInvalid()
		}
		value = field.Default
	}
	if !behaviorPropertyValueAllowed(field, value) {
		return nil, behaviorInvalid()
	}
	return value, nil
}

func behaviorParseProperty(kind string, raw string, integer bool) (BehaviorValue, error) {
	switch kind {
	case "string":
		return raw, nil
	case "boolean":
		if raw == "true" {
			return true, nil
		}
		if raw == "false" {
			return false, nil
		}
		return nil, behaviorInvalid()
	}
	if !behaviorNumberToken(raw, integer) {
		return nil, behaviorInvalid()
	}
	number, err := strconv.ParseFloat(raw, 64)
	if err != nil {
		return nil, behaviorInvalid()
	}
	return number, nil
}

func behaviorPropertyValueAllowed(field BehaviorPropertyField, value BehaviorValue) bool {
	switch typed := value.(type) {
	case string:
		return field.Kind == "string" && len(typed) <= 4096
	case bool:
		return field.Kind == "boolean"
	case float64:
		numeric := field.Kind == "integer" || field.Kind == "number"
		if !numeric || !behaviorPropertyNumber(typed, field.Kind == "integer") {
			return false
		}
		if field.HasMin && typed < field.Min {
			return false
		}
		return !field.HasMax || typed <= field.Max
	}
	return false
}

func behaviorBindProperties(policy BehaviorPropertiesPolicy, values map[string]string) (BehaviorValue, error) {
	if err := behaviorValidateProperties(values); err != nil {
		return nil, err
	}
	validPrefix := behaviorEnvironmentName(policy.Prefix) && len(policy.Prefix) <= 128 && strings.HasPrefix(policy.Prefix, behaviorFrameworkPrefix()) && strings.HasSuffix(policy.Prefix, "_")
	if !validPrefix || len(policy.Fields) == 0 || len(policy.Fields) > 64 {
		return nil, behaviorInvalid()
	}
	names := map[string]bool{}
	output := map[string]any{}
	for _, field := range policy.Fields {
		name := policy.Prefix + field.Name
		_, taken := output[field.Field]
		if !behaviorIdentifier(field.Field) || !behaviorEnvironmentName(field.Name) || len(name) > 256 || names[name] || taken {
			return nil, behaviorInvalid()
		}
		names[name] = true
		raw, present := values[name]
		value, err := behaviorPropertyValue(field, raw, present)
		if err != nil {
			return nil, err
		}
		output[field.Field] = value
	}
	for name := range values {
		if strings.HasPrefix(name, policy.Prefix) && !names[name] {
			return nil, behaviorInvalid()
		}
	}
	return behaviorChecked(output, 65536)
}

func behaviorApplicationInputs(application BehaviorValue) (map[string]string, error) {
	checked, err := behaviorChecked(application, 65536)
	if err != nil {
		return nil, err
	}
	root, isObject := checked.(map[string]any)
	if !isObject {
		return nil, behaviorInvalid()
	}
	_, hasTac := root["tac"]
	_, hasYon := root["yon"]
	if !hasTac && !hasYon {
		return nil, behaviorInvalid()
	}
	flat := map[string]string{}
	for _, key := range []string{"tac", "yon"} {
		value, present := root[key]
		if !present {
			continue
		}
		if _, isObject := value.(map[string]any); !isObject {
			return nil, behaviorInvalid()
		}
		if err := behaviorFlattenApplication(value, key, 0, flat); err != nil {
			return nil, err
		}
	}
	return flat, nil
}

func behaviorFlattenApplication(value BehaviorValue, path string, depth int, flat map[string]string) error {
	if depth > 16 || len(path) > 256 || len(flat) >= 256 {
		return behaviorInvalid()
	}
	if path == "yon.server.port" {
		number, isNumber := value.(float64)
		if !isNumber || !behaviorPropertyNumber(number, true) {
			return behaviorInvalid()
		}
	}
	values, isObject := value.(map[string]any)
	if !isObject {
		return behaviorFlattenScalar(value, path, flat)
	}
	if depth > 0 && len(values) == 0 {
		flat[path+"."] = ""
	}
	for _, key := range behaviorSortedKeys(values) {
		if !behaviorApplicationKey(key) {
			return behaviorInvalid()
		}
		if err := behaviorFlattenApplication(values[key], path+"."+key, depth+1, flat); err != nil {
			return err
		}
	}
	return nil
}

func behaviorSortedKeys(values map[string]any) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	for index := 1; index < len(keys); index++ {
		for back := index; back > 0 && keys[back] < keys[back-1]; back-- {
			keys[back], keys[back-1] = keys[back-1], keys[back]
		}
	}
	return keys
}

func behaviorApplicationKey(key string) bool {
	if key == "" || key[0] < 'a' || key[0] > 'z' {
		return false
	}
	for index := 0; index < len(key); index++ {
		current := key[index]
		if !(current >= 'a' && current <= 'z') && !(current >= '0' && current <= '9') {
			return false
		}
	}
	return true
}

func behaviorFlattenScalar(value BehaviorValue, path string, flat map[string]string) error {
	text := ""
	switch typed := value.(type) {
	case nil, []any:
		flat[path+"."] = ""
		return nil
	case string:
		text = typed
	case bool:
		text = strconv.FormatBool(typed)
	case float64:
		text = strconv.FormatFloat(typed, 'f', -1, 64)
	default:
		return behaviorInvalid()
	}
	if len(text) > 4096 || len(flat) >= 256 {
		return behaviorInvalid()
	}
	flat[path] = text
	return nil
}

func behaviorPropertyInputs(input any) (map[string]string, error) {
	if values, isMap := input.(map[string]string); isMap {
		if err := behaviorValidateProperties(values); err != nil {
			return nil, err
		}
		copied := map[string]string{}
		for key, value := range values {
			copied[key] = value
		}
		return copied, nil
	}
	return behaviorApplicationProperties(input)
}

func behaviorApplicationProperties(application BehaviorValue) (map[string]string, error) {
	flat, err := behaviorApplicationInputs(application)
	if err != nil {
		return nil, err
	}
	prefix := behaviorEnvironment + "."
	properties := map[string]string{}
	for path, value := range flat {
		if strings.HasPrefix(path, prefix) {
			properties[strings.ToUpper(strings.ReplaceAll(path, ".", "_"))] = value
		}
	}
	return properties, nil
}

func behaviorBindValues(policy BehaviorValuesPolicy, application BehaviorValue) (BehaviorValue, error) {
	if len(policy.Fields) == 0 || len(policy.Fields) > 64 {
		return nil, behaviorInvalid()
	}
	values, err := behaviorApplicationInputs(application)
	if err != nil {
		return nil, err
	}
	output := map[string]any{}
	for _, field := range policy.Fields {
		if _, taken := output[field.Field]; taken || !behaviorIdentifier(field.Field) {
			return nil, behaviorInvalid()
		}
		for path := range values {
			if strings.HasPrefix(path, field.Name+".") {
				return nil, behaviorInvalid()
			}
		}
		raw, present := values[field.Name]
		value, err := behaviorPropertyValue(field, raw, present)
		if err != nil {
			return nil, err
		}
		output[field.Field] = value
	}
	return behaviorChecked(output, 65536)
}

func behaviorLoadConfiguration(source behaviorConfigurationSource, execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration, err := source.behaviorConfigure(execution)
	if err != nil || configuration == nil {
		err = behaviorError("adapter", "Behavior configuration failed.")
	}
	if checkError := execution.Check(); checkError != nil {
		return nil, checkError
	}
	if err != nil {
		return nil, err
	}
	names := []string{}
	for name := range configuration.Caches {
		names = append(names, name)
	}
	if behaviorAdapterNames(names) != nil {
		return nil, behaviorInvalid()
	}
	names = []string{}
	for name := range configuration.Http {
		names = append(names, name)
	}
	if behaviorAdapterNames(names) != nil {
		return nil, behaviorInvalid()
	}
	names = []string{}
	for name := range configuration.Entities {
		names = append(names, name)
	}
	if behaviorAdapterNames(names) != nil {
		return nil, behaviorInvalid()
	}
	return configuration, nil
}

func behaviorAdapterNames(names []string) error {
	if len(names) > 128 {
		return behaviorInvalid()
	}
	for _, name := range names {
		if !behaviorIdentifier(name) {
			return behaviorInvalid()
		}
	}
	return nil
}

func behaviorNewExecution(options *BehaviorOptions, disposed *BehaviorSignal) *BehaviorExecution {
	timeout := time.Duration(options.TimeoutMs) * time.Millisecond
	return &BehaviorExecution{
		Environment: behaviorEnvironment,
		Partition:   options.Partition,
		Deadline:    behaviorNow() + options.TimeoutMs,
		owner:       options.Signal,
		disposed:    disposed,
		until:       time.Now().Add(timeout),
	}
}

func behaviorNow() int64 {
	return time.Now().UnixMilli()
}

func behaviorIdentifier(value string) bool {
	if value == "" || len(value) > 128 {
		return false
	}
	for index := 0; index < len(value); index++ {
		current := value[index]
		letter := (current >= 'a' && current <= 'z') || (current >= 'A' && current <= 'Z')
		digit := index > 0 && current >= '0' && current <= '9'
		if !letter && !digit && current != '_' && current != '$' {
			return false
		}
	}
	switch value {
	case "__proto__", "prototype", "constructor", "behaviors", "tac":
		return false
	}
	return true
}

type BehaviorContext struct {
	options       *BehaviorOptions
	configuration *BehaviorConfiguration
	disposed      *BehaviorSignal
	active        atomic.Int64
	lock          sync.Mutex
	slots         map[string]int
	transaction   string
}

type behaviorOperation struct {
	context   *BehaviorContext
	execution *BehaviorExecution
}

func behaviorAdapterCall[T any](execution *BehaviorExecution, call func() (T, error)) (T, error) {
	var empty T
	if err := execution.Check(); err != nil {
		return empty, err
	}
	value, err := call()
	if err != nil && !behaviorStorageFailure(err) {
		err = behaviorError("adapter", "Behavior adapter failed.")
	}
	if checkError := execution.Check(); checkError != nil {
		return empty, checkError
	}
	if err != nil {
		return empty, err
	}
	return value, nil
}

func (context *BehaviorContext) cache(name string) (BehaviorCacheAdapter, bool) {
	if adapter, present := context.configuration.Caches[name]; present {
		return adapter, true
	}
	return behaviorBuiltinCache(name)
}

func (context *BehaviorContext) builtinCache(name string) bool {
	_, configured := context.configuration.Caches[name]
	_, builtin := behaviorBuiltinCache(name)
	return !configured && builtin
}

func (context *BehaviorContext) entities(name string) (BehaviorEntityAdapter, bool) {
	if adapter, present := context.configuration.Entities[name]; present {
		return adapter, true
	}
	return behaviorBuiltinEntities(name)
}

func (context *BehaviorContext) builtinEntities(name string) bool {
	_, configured := context.configuration.Entities[name]
	_, builtin := behaviorBuiltinEntities(name)
	return !configured && builtin
}

func behaviorAllowEntity(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
	return true, nil
}

func behaviorStorageFailure(err error) bool {
	var failure *BehaviorError
	return errors.As(err, &failure) && failure.Code == "storage"
}

func (context *BehaviorContext) operation() (*behaviorOperation, error) {
	execution := behaviorNewExecution(context.options, context.disposed)
	if err := execution.Check(); err != nil {
		return nil, err
	}
	if context.active.Add(1) > 64 {
		context.active.Add(-1)
		return nil, behaviorError("capacity", "Behavior context concurrency bound reached.")
	}
	return &behaviorOperation{context: context, execution: execution}, nil
}

func (operation *behaviorOperation) finish() {
	operation.context.active.Add(-1)
}

func (context *BehaviorContext) Dispose() {
	context.disposed.Abort()
}

func (context *BehaviorContext) RestClient(name string) (*BehaviorRestClient, error) {
	if !behaviorIdentifier(name) {
		return nil, behaviorInvalid()
	}
	return &BehaviorRestClient{context: context, name: name}, nil
}

func (context *BehaviorContext) Invoke(policy BehaviorMethodPolicy, input []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	operation, err := context.operation()
	if err != nil {
		return nil, err
	}
	defer operation.finish()
	args, err := behaviorArguments(input, policy)
	if err != nil {
		return nil, err
	}
	var value BehaviorValue
	switch {
	case policy.Mutation != nil:
		value, err = context.mutate(operation, policy, args, original)
	case policy.KeyArgs != nil:
		return context.cached(operation, policy, args, original)
	default:
		value, err = context.result(operation, policy, args, original)
	}
	if err != nil {
		return nil, err
	}
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	return value, nil
}

func (context *BehaviorContext) cached(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	cache := policy.Cache
	if cache == nil || behaviorValidateCache(cache) != nil || behaviorHttpMethod(policy.Name) {
		return nil, behaviorInvalid()
	}
	selected, err := behaviorSelected(policy.KeyArgs, args)
	if err != nil {
		return nil, err
	}
	if err := context.authorize(operation, policy, selected, "cache:read"); err != nil {
		return nil, err
	}
	key, err := context.cacheKey(policy, selected)
	if err != nil {
		return nil, err
	}
	adapter, present := context.cache(cache.Name)
	if !present {
		return nil, behaviorInvalid()
	}
	entry, err := behaviorAdapterCall(operation.execution, func() (*BehaviorCacheEntry, error) {
		return adapter.Get(key, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	fresh, hit, err := behaviorFreshEntry(entry, cache)
	if err != nil {
		return nil, err
	}
	if hit {
		if err := operation.execution.Check(); err != nil {
			return nil, err
		}
		return fresh, nil
	}
	value, err := context.result(operation, policy, args, original)
	if err != nil {
		return nil, err
	}
	value, err = behaviorChecked(value, cache.MaxBytes)
	if err != nil {
		return nil, err
	}
	write := &BehaviorCacheWrite{Execution: operation.execution, Policy: *cache, ExpiresAt: behaviorNow() + cache.TtlMs}
	_, err = behaviorAdapterCall(operation.execution, func() (bool, error) {
		return true, adapter.Set(key, value, write)
	})
	if err != nil {
		return nil, err
	}
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	return value, nil
}

func (context *BehaviorContext) mutate(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	mutation := policy.Mutation
	validOperation := mutation.Operation == "cache:put" || mutation.Operation == "cache:evict"
	if policy.KeyArgs != nil || !validOperation || !behaviorIdentifier(mutation.Method) || behaviorHttpMethod(policy.Name) || len(mutation.Arguments) > 32 {
		return nil, behaviorInvalid()
	}
	cache := policy.Cache
	if cache == nil || behaviorValidateCache(cache) != nil {
		return nil, behaviorInvalid()
	}
	target := behaviorMutationTarget(policy)
	mapped, err := behaviorMutationArguments(mutation, args)
	if err != nil {
		return nil, err
	}
	targetArgs, err := behaviorArguments(mapped, target)
	if err != nil {
		return nil, err
	}
	selected, err := behaviorSelected(mutation.KeyArgs, targetArgs)
	if err != nil {
		return nil, err
	}
	if err := context.authorize(operation, target, selected, mutation.Operation); err != nil {
		return nil, err
	}
	key, err := context.cacheKey(target, selected)
	if err != nil {
		return nil, err
	}
	adapter, present := context.cache(cache.Name)
	if !present {
		return nil, behaviorInvalid()
	}
	limit := 1048576
	if mutation.Operation == "cache:put" {
		limit = cache.MaxBytes
	}
	value, err := context.result(operation, policy, args, original)
	if err != nil {
		return nil, err
	}
	value, err = behaviorChecked(value, limit)
	if err != nil {
		return nil, err
	}
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	if mutation.Operation == "cache:put" {
		write := &BehaviorCacheWrite{Execution: operation.execution, Policy: *cache, ExpiresAt: behaviorNow() + cache.TtlMs}
		_, err = behaviorAdapterCall(operation.execution, func() (bool, error) {
			return true, adapter.Set(key, value, write)
		})
	} else {
		_, err = behaviorAdapterCall(operation.execution, func() (bool, error) {
			return true, adapter.Delete(key, operation.execution)
		})
	}
	if err != nil {
		return nil, err
	}
	return value, nil
}

func (context *BehaviorContext) result(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	if policy.Limit == nil {
		return context.attempt(operation, policy, args, original)
	}
	key := policy.Owner + "." + policy.Name
	if err := context.acquireSlot(operation, key, policy.Limit); err != nil {
		return nil, err
	}
	defer context.releaseSlot(key)
	return context.attempt(operation, policy, args, original)
}

func (context *BehaviorContext) acquireSlot(operation *behaviorOperation, key string, limit *BehaviorLimitPolicy) error {
	for {
		context.lock.Lock()
		if context.slots[key] < limit.Limit {
			context.slots[key]++
			context.lock.Unlock()
			return nil
		}
		context.lock.Unlock()
		if !limit.Blocking {
			return behaviorError("capacity", "Behavior method concurrency limit reached.")
		}
		if err := operation.execution.Check(); err != nil {
			return err
		}
		time.Sleep(5 * time.Millisecond)
	}
}

func (context *BehaviorContext) releaseSlot(key string) {
	context.lock.Lock()
	defer context.lock.Unlock()
	context.slots[key]--
	if context.slots[key] <= 0 {
		delete(context.slots, key)
	}
}

func (context *BehaviorContext) joined() string {
	context.lock.Lock()
	defer context.lock.Unlock()
	return context.transaction
}

func (context *BehaviorContext) setTransaction(id string) {
	context.lock.Lock()
	defer context.lock.Unlock()
	context.transaction = id
}

func (context *BehaviorContext) transacted(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	if policy.Transactional == nil || context.joined() != "" {
		return context.execute(operation, policy, args, original)
	}
	adapter, present := context.entities(policy.Transactional.Adapter)
	if !present {
		return nil, behaviorInvalid()
	}
	control := func(action string, id string) (BehaviorValue, error) {
		transactions, supported := adapter.(BehaviorTransactionAdapter)
		if !supported {
			return nil, behaviorError("adapter", "This entity adapter has no transactions.")
		}
		command := &BehaviorTransactionCommand{Action: action, Transaction: id}
		return behaviorAdapterCall(operation.execution, func() (BehaviorValue, error) {
			return transactions.Transaction(command, operation.execution)
		})
	}
	begun, err := control("begin", "")
	if err != nil {
		return nil, err
	}
	id, isText := begun.(string)
	if !isText || id == "" || len(id) > 128 {
		return nil, behaviorInvalid()
	}
	context.setTransaction(id)
	value, err := context.execute(operation, policy, args, original)
	context.setTransaction("")
	if err != nil {
		control("rollback", id)
		return nil, err
	}
	if _, err := control("commit", id); err != nil {
		return nil, err
	}
	return value, nil
}

func (context *BehaviorContext) attempt(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	retry := policy.Retry
	if retry == nil {
		return context.transacted(operation, policy, args, original)
	}
	for attempt := 0; attempt <= retry.MaxRetries; attempt++ {
		copied := append([]BehaviorValue{}, args...)
		value, err := context.transacted(operation, policy, copied, original)
		if err == nil {
			return value, nil
		}
		if attempt == retry.MaxRetries || !behaviorRetryable(err) {
			return nil, err
		}
		if err := operation.execution.Check(); err != nil {
			return nil, err
		}
		if wait := behaviorBackoff(retry, attempt); wait > 0 {
			time.Sleep(wait)
		}
		if err := operation.execution.Check(); err != nil {
			return nil, err
		}
	}
	return nil, behaviorInvalid()
}

func (context *BehaviorContext) execute(operation *behaviorOperation, policy BehaviorMethodPolicy, args []BehaviorValue, original func([]BehaviorValue) (BehaviorValue, error)) (BehaviorValue, error) {
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	http := policy.Http
	if http != nil && len(http.PathVariables) > 0 {
		filled, pathError := behaviorExchangePath(http, args)
		if pathError != nil {
			return nil, pathError
		}
		http = filled
	}
	declared := http != nil && (len(http.Query) > 0 || len(http.Headers) > 0 || http.HasBody || len(http.PathVariables) > 0)
	var value BehaviorValue
	var err error
	if declared {
		value, err = behaviorExchangeRequest(http, args)
	} else {
		value, err = original(args)
	}
	if checkError := operation.execution.Check(); checkError != nil {
		return nil, checkError
	}
	if err != nil {
		return nil, err
	}
	if http == nil {
		return behaviorChecked(value, 1048576)
	}
	request, err := behaviorHttpRequest(http, value)
	if err != nil {
		return nil, err
	}
	adapter, present := context.configuration.Http[http.Client]
	if !present {
		return nil, behaviorInvalid()
	}
	answer, err := behaviorAdapterCall(operation.execution, func() (BehaviorValue, error) {
		return adapter.Exchange(request, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	return behaviorChecked(answer, 1048576)
}

func (context *BehaviorContext) authorize(operation *behaviorOperation, policy BehaviorMethodPolicy, selected []BehaviorValue, intent string) error {
	authorize := context.configuration.Authorize
	if policy.Cache == nil {
		return behaviorInvalid()
	}
	if authorize == nil && context.builtinCache(policy.Cache.Name) {
		return nil
	}
	if authorize == nil {
		return behaviorInvalid()
	}
	call := &BehaviorAuthorizationCall{Operation: intent, Cache: policy.Cache.Name, Owner: policy.Owner, Method: policy.Name, KeyArguments: selected}
	allowed, err := behaviorAdapterCall(operation.execution, func() (bool, error) {
		return authorize(call, operation.execution)
	})
	if err != nil {
		return err
	}
	if !allowed {
		return behaviorError("unauthorized", "Behavior cache access was not authorized.")
	}
	return nil
}

func (context *BehaviorContext) cacheKey(policy BehaviorMethodPolicy, selected []BehaviorValue) (string, error) {
	if policy.Cache == nil {
		return "", behaviorInvalid()
	}
	key := []any{context.options.Partition, policy.Cache.Name, behaviorModuleID, policy.Owner, policy.Name, selected}
	return behaviorEncode(key, 4096)
}

func (context *BehaviorContext) Evict(service behaviorService, method string, input []BehaviorValue) error {
	if service.behaviorContext() != context {
		return behaviorInvalid()
	}
	policy := service.behaviorMethod(method)
	if policy == nil || policy.KeyArgs == nil {
		return behaviorInvalid()
	}
	operation, err := context.operation()
	if err != nil {
		return err
	}
	defer operation.finish()
	args, err := behaviorArguments(input, *policy)
	if err != nil {
		return err
	}
	selected, err := behaviorSelected(policy.KeyArgs, args)
	if err != nil {
		return err
	}
	if err := context.authorize(operation, *policy, selected, "cache:evict"); err != nil {
		return err
	}
	key, err := context.cacheKey(*policy, selected)
	if err != nil {
		return err
	}
	adapter, present := context.cache(policy.Cache.Name)
	if !present {
		return behaviorInvalid()
	}
	_, err = behaviorAdapterCall(operation.execution, func() (bool, error) {
		return true, adapter.Delete(key, operation.execution)
	})
	return err
}

func (context *BehaviorContext) Query(policy BehaviorQueryPolicy, input []BehaviorValue) (BehaviorValue, error) {
	operation, err := context.operation()
	if err != nil {
		return nil, err
	}
	defer operation.finish()
	args, err := behaviorCheckedList(input)
	if err != nil {
		return nil, err
	}
	query, err := behaviorQuery(policy, args)
	if err != nil {
		return nil, err
	}
	authorize := context.configuration.AuthorizeEntity
	if authorize == nil && !context.builtinEntities(policy.Entity.Adapter) {
		return nil, behaviorInvalid()
	}
	if authorize == nil {
		authorize = behaviorAllowEntity
	}
	adapter, present := context.entities(policy.Entity.Adapter)
	if !present {
		return nil, behaviorInvalid()
	}
	command := &BehaviorEntityCommand{Transaction: context.joined(), Operation: "query", Entity: policy.Entity.Name, Table: policy.Entity.Table, IdField: policy.Entity.Id, Query: query}
	if err := behaviorCheckQueryCommand(command); err != nil {
		return nil, err
	}
	allowed, err := behaviorAdapterCall(operation.execution, func() (bool, error) {
		return authorize(command, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	if !allowed {
		return nil, behaviorError("unauthorized", "Store query was not authorized.")
	}
	result, err := behaviorAdapterCall(operation.execution, func() (BehaviorValue, error) {
		return adapter.Execute(command, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	result, err = behaviorChecked(result, 1048576)
	if err != nil {
		return nil, err
	}
	if err := behaviorCheckQueryResult(policy.Operation, command, result); err != nil {
		return nil, err
	}
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	return result, nil
}

func behaviorCheckedList(values []BehaviorValue) ([]BehaviorValue, error) {
	checked := make([]BehaviorValue, 0, len(values))
	for _, value := range values {
		normalized, err := behaviorChecked(value, 1048576)
		if err != nil {
			return nil, err
		}
		checked = append(checked, normalized)
	}
	return checked, nil
}

func behaviorCheckQueryCommand(command *BehaviorEntityCommand) error {
	if command.Query == nil {
		return behaviorInvalid()
	}
	shape := map[string]any{"operation": command.Operation, "entity": command.Entity, "table": command.Table, "idField": command.IdField, "query": command.Query}
	_, err := behaviorEncode(shape, 1048576)
	return err
}

func behaviorCheckQueryResult(operation string, command *BehaviorEntityCommand, result BehaviorValue) error {
	switch operation {
	case "find":
		var size BehaviorValue
		if query, isObject := command.Query.(map[string]any); isObject {
			if page, isPage := query["page"].(map[string]any); isPage {
				size = page["size"]
			}
		}
		limit, err := behaviorInteger(size, size != nil, 0, 1, 100)
		if err != nil {
			return err
		}
		rows, isList := result.([]any)
		if !isList || int64(len(rows)) > limit {
			return behaviorInvalid()
		}
		for _, row := range rows {
			if _, isObject := row.(map[string]any); !isObject {
				return behaviorInvalid()
			}
		}
		return nil
	case "count":
		_, err := behaviorInteger(result, true, 0, 0, 9007199254740991)
		return err
	case "exists":
		if _, isBool := result.(bool); !isBool {
			return behaviorInvalid()
		}
		return nil
	}
	return behaviorInvalid()
}

func behaviorHttpMethod(name string) bool {
	switch name {
	case "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS":
		return true
	}
	return false
}

func behaviorFreshEntry(entry *BehaviorCacheEntry, cache *BehaviorCachePolicy) (BehaviorValue, bool, error) {
	if entry == nil {
		return nil, false, nil
	}
	now := behaviorNow()
	if entry.ExpiresAt > now+cache.TtlMs {
		return nil, false, behaviorInvalid()
	}
	if entry.ExpiresAt <= now {
		return nil, false, nil
	}
	value, err := behaviorChecked(entry.Value, cache.MaxBytes)
	if err != nil {
		return nil, false, err
	}
	return value, true, nil
}

func behaviorMutationTarget(policy BehaviorMethodPolicy) BehaviorMethodPolicy {
	target := policy
	target.Name = policy.Mutation.Method
	target.KeyArgs = append([]int{}, policy.Mutation.KeyArgs...)
	target.Pageable = policy.Mutation.Pageable
	target.Mutation = nil
	target.Http = nil
	return target
}

func behaviorMutationArguments(mutation *BehaviorCacheMutationPolicy, args []BehaviorValue) ([]BehaviorValue, error) {
	mapped := []BehaviorValue{}
	for _, index := range mutation.Arguments {
		if index < 0 || index >= 32 || index >= len(args) {
			return nil, behaviorInvalid()
		}
		mapped = append(mapped, args[index])
	}
	return mapped, nil
}

func behaviorValidateCache(policy *BehaviorCachePolicy) error {
	if !behaviorIdentifier(policy.Name) || policy.TtlMs < 1 || policy.TtlMs > 86400000 || policy.MaxBytes < 1 || policy.MaxBytes > 1048576 || policy.MaxEntries < 1 || policy.MaxEntries > 10000 {
		return behaviorInvalid()
	}
	return nil
}

func behaviorSelected(indices []int, input []BehaviorValue) ([]BehaviorValue, error) {
	if len(indices) > 32 {
		return nil, behaviorInvalid()
	}
	seen := map[int]bool{}
	values := []BehaviorValue{}
	for _, index := range indices {
		if index < 0 || index >= 32 || seen[index] || index >= len(input) {
			return nil, behaviorInvalid()
		}
		seen[index] = true
		value, err := behaviorChecked(input[index], 4096)
		if err != nil {
			return nil, err
		}
		values = append(values, value)
	}
	if _, err := behaviorEncode(values, 4096); err != nil {
		return nil, err
	}
	return values, nil
}

func behaviorArguments(input []BehaviorValue, policy BehaviorMethodPolicy) ([]BehaviorValue, error) {
	if len(input) > 64 {
		return nil, behaviorInvalid()
	}
	args, err := behaviorCheckedList(input)
	if err != nil {
		return nil, err
	}
	page := policy.Pageable
	if page == nil {
		return args, nil
	}
	if page.Argument < 0 || page.Argument >= len(args) {
		return nil, behaviorInvalid()
	}
	normalized, err := behaviorPage(args[page.Argument], page)
	if err != nil {
		return nil, err
	}
	args[page.Argument] = normalized
	return args, nil
}

func behaviorInteger(value BehaviorValue, present bool, fallback int64, minimum int64, maximum int64) (int64, error) {
	if !present {
		return fallback, nil
	}
	number, isNumber := value.(float64)
	if !isNumber || math.IsNaN(number) || math.IsInf(number, 0) || number != math.Trunc(number) || number < float64(minimum) || number > float64(maximum) {
		return 0, behaviorInvalid()
	}
	return int64(number), nil
}

func behaviorPage(input BehaviorValue, policy *BehaviorPagePolicy) (BehaviorValue, error) {
	if behaviorInvalidPagePolicy(policy) {
		return nil, behaviorInvalid()
	}
	checked, err := behaviorChecked(input, 4096)
	if err != nil {
		return nil, err
	}
	values, isObject := checked.(map[string]any)
	if !isObject {
		return nil, behaviorInvalid()
	}
	for key := range values {
		if key != "page" && key != "size" && key != "sort" && key != "offset" {
			return nil, behaviorInvalid()
		}
	}
	pageValue, hasPage := values["page"]
	page, err := behaviorInteger(pageValue, hasPage, 0, 0, 1000000)
	if err != nil {
		return nil, err
	}
	sizeValue, hasSize := values["size"]
	size, err := behaviorInteger(sizeValue, hasSize, policy.DefaultSize, 1, policy.MaxSize)
	if err != nil {
		return nil, err
	}
	offset := page * size
	if offset > 1000000 {
		return nil, behaviorInvalid()
	}
	offsetValue, hasOffset := values["offset"]
	declared, err := behaviorInteger(offsetValue, hasOffset, offset, 0, 1000000)
	if err != nil || declared != offset {
		return nil, behaviorInvalid()
	}
	sortValue, hasSort := values["sort"]
	sort, err := behaviorPageSort(sortValue, hasSort, policy)
	if err != nil {
		return nil, err
	}
	return map[string]any{"page": float64(page), "size": float64(size), "offset": float64(offset), "sort": sort}, nil
}

func behaviorInvalidPagePolicy(policy *BehaviorPagePolicy) bool {
	if policy.Argument < 0 || policy.Argument >= 32 || policy.DefaultSize < 1 || policy.DefaultSize > policy.MaxSize || policy.MaxSize > 1000 || len(policy.Sort) > 32 {
		return true
	}
	for _, name := range policy.Sort {
		if !behaviorIdentifier(name) {
			return true
		}
	}
	return false
}

func behaviorPageSort(value BehaviorValue, present bool, policy *BehaviorPagePolicy) ([]any, error) {
	normalized := []any{}
	if !present {
		return normalized, nil
	}
	entries, isList := value.([]any)
	if !isList || len(entries) > 16 {
		return nil, behaviorInvalid()
	}
	seen := map[string]bool{}
	for _, entry := range entries {
		text, isText := entry.(string)
		if !isText {
			return nil, behaviorInvalid()
		}
		parts := strings.Split(text, ":")
		field := parts[0]
		direction := "asc"
		if len(parts) > 1 {
			direction = parts[1]
		}
		allowed := false
		for _, name := range policy.Sort {
			if name == field {
				allowed = true
			}
		}
		if len(parts) > 2 || !allowed || (direction != "asc" && direction != "desc") || seen[field] {
			return nil, behaviorInvalid()
		}
		seen[field] = true
		normalized = append(normalized, field+":"+direction)
	}
	return normalized, nil
}

type BehaviorRestClient struct {
	context *BehaviorContext
	name    string
}

func (client *BehaviorRestClient) Get(path string) *BehaviorRestCall {
	return client.call("GET", path)
}

func (client *BehaviorRestClient) Post(path string) *BehaviorRestCall {
	return client.call("POST", path)
}

func (client *BehaviorRestClient) Put(path string) *BehaviorRestCall {
	return client.call("PUT", path)
}

func (client *BehaviorRestClient) Patch(path string) *BehaviorRestCall {
	return client.call("PATCH", path)
}

func (client *BehaviorRestClient) Delete(path string) *BehaviorRestCall {
	return client.call("DELETE", path)
}

func (client *BehaviorRestClient) Head(path string) *BehaviorRestCall {
	return client.call("HEAD", path)
}

func (client *BehaviorRestClient) Options(path string) *BehaviorRestCall {
	return client.call("OPTIONS", path)
}

func (client *BehaviorRestClient) call(method string, path string) *BehaviorRestCall {
	return &BehaviorRestCall{context: client.context, name: client.name, method: method, path: path, query: map[string]any{}, headers: map[string]any{}}
}

type BehaviorRestCall struct {
	context *BehaviorContext
	name    string
	method  string
	path    string
	query   map[string]any
	headers map[string]any
	body    BehaviorValue
	hasBody bool
}

func (call *BehaviorRestCall) Query(name string, value BehaviorValue) *BehaviorRestCall {
	call.query[name] = value
	return call
}

func (call *BehaviorRestCall) Header(name string, value string) *BehaviorRestCall {
	call.headers[name] = value
	return call
}

func (call *BehaviorRestCall) Body(value BehaviorValue) *BehaviorRestCall {
	call.body = value
	call.hasBody = true
	return call
}

func (call *BehaviorRestCall) Retrieve() (BehaviorValue, error) {
	if call.hasBody && (call.method == "GET" || call.method == "HEAD") {
		return nil, behaviorInvalid()
	}
	descriptor := map[string]any{"query": call.query, "headers": call.headers}
	if call.hasBody {
		descriptor["body"] = call.body
	}
	operation, err := call.context.operation()
	if err != nil {
		return nil, err
	}
	defer operation.finish()
	if !behaviorSafePath(call.path) {
		return nil, behaviorInvalid()
	}
	policy := &BehaviorHttpPolicy{Client: call.name, Method: call.method, Path: "/"}
	request, err := behaviorHttpRequest(policy, descriptor)
	if err != nil {
		return nil, err
	}
	request.Path = call.path
	adapter, present := call.context.configuration.Http[call.name]
	if !present {
		return nil, behaviorInvalid()
	}
	answer, err := behaviorAdapterCall(operation.execution, func() (BehaviorValue, error) {
		return adapter.Exchange(request, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	return behaviorChecked(answer, 1048576)
}

type BehaviorRepository struct {
	context  *BehaviorContext
	metadata BehaviorEntityMetadata
}

func (repository *BehaviorRepository) execute(operationName string, value BehaviorValue, page BehaviorValue, id BehaviorValue) (BehaviorValue, error) {
	operation, err := repository.context.operation()
	if err != nil {
		return nil, err
	}
	defer operation.finish()
	adapter, present := repository.context.entities(repository.metadata.Adapter)
	if !present {
		return nil, behaviorInvalid()
	}
	command := &BehaviorEntityCommand{
		Transaction: repository.context.joined(),
		Entity:      repository.metadata.Name,
		Table:       repository.metadata.Table,
		IdField:     repository.metadata.Id,
		Operation:   operationName,
		Value:       value,
		Page:        page,
		Id:          id,
	}
	result, err := behaviorAdapterCall(operation.execution, func() (BehaviorValue, error) {
		return adapter.Execute(command, operation.execution)
	})
	if err != nil {
		return nil, err
	}
	return behaviorChecked(result, 1048576)
}

func (repository *BehaviorRepository) Find(page BehaviorValue) (BehaviorValue, error) {
	policy := &BehaviorPagePolicy{Argument: 0, DefaultSize: 20, MaxSize: 100, Sort: []string{repository.metadata.Id}}
	normalized, err := behaviorPage(page, policy)
	if err != nil {
		return nil, err
	}
	return repository.execute("find", nil, normalized, nil)
}

func (repository *BehaviorRepository) Save(value BehaviorValue) (BehaviorValue, error) {
	checked, err := behaviorChecked(value, 1048576)
	if err != nil {
		return nil, err
	}
	object, isObject := checked.(map[string]any)
	if !isObject {
		return nil, behaviorInvalid()
	}
	if err := behaviorEntityId(object[repository.metadata.Id]); err != nil {
		return nil, err
	}
	return repository.execute("save", checked, nil, nil)
}

func (repository *BehaviorRepository) Delete(id BehaviorValue) (BehaviorValue, error) {
	checked, err := behaviorChecked(id, 1048576)
	if err != nil {
		return nil, err
	}
	if err := behaviorEntityId(checked); err != nil {
		return nil, err
	}
	return repository.execute("delete", nil, nil, checked)
}

func behaviorQuery(policy BehaviorQueryPolicy, args []BehaviorValue) (BehaviorValue, error) {
	if behaviorInvalidQueryPolicy(policy, len(args)) {
		return nil, behaviorInvalid()
	}
	used := map[int]bool{}
	predicates := []any{}
	for _, predicate := range policy.Predicates {
		value, err := behaviorQueryPredicate(policy, predicate, args, used)
		if err != nil {
			return nil, err
		}
		predicates = append(predicates, value)
	}
	checked, err := behaviorChecked(predicates, 65536)
	if err != nil {
		return nil, err
	}
	order, err := behaviorQueryOrder(policy)
	if err != nil {
		return nil, err
	}
	query := map[string]any{"kind": policy.Operation, "where": checked, "orderBy": order}
	if policy.Operation == "find" {
		page, err := behaviorQueryPage(policy, args, used)
		if err != nil {
			return nil, err
		}
		query["page"] = page
	}
	if len(used) != len(args) {
		return nil, behaviorInvalid()
	}
	return query, nil
}

func behaviorInvalidQueryPolicy(policy BehaviorQueryPolicy, argumentCount int) bool {
	entity := policy.Entity
	for _, name := range []string{entity.Name, entity.Adapter, entity.Table, entity.Id} {
		if !behaviorIdentifier(name) {
			return true
		}
	}
	if len(policy.Fields) == 0 || len(policy.Fields) > 64 {
		return true
	}
	unique := map[string]bool{}
	for _, field := range policy.Fields {
		if !behaviorIdentifier(field) || unique[field] {
			return true
		}
		unique[field] = true
	}
	if !unique[entity.Id] {
		return true
	}
	if policy.DefaultSize < 1 || policy.DefaultSize > policy.MaxSize || policy.MaxSize > 100 {
		return true
	}
	if len(policy.Predicates) > 8 || len(policy.OrderBy) > 5 || argumentCount > 32 {
		return true
	}
	switch policy.Operation {
	case "find":
		return false
	case "count", "exists":
		return policy.HasPageArgument || len(policy.OrderBy) > 0
	}
	return true
}

func behaviorQueryPredicate(policy BehaviorQueryPolicy, predicate BehaviorQueryPredicate, args []BehaviorValue, used map[int]bool) (BehaviorValue, error) {
	known := false
	for _, field := range policy.Fields {
		if field == predicate.Field {
			known = true
		}
	}
	if !known {
		return nil, behaviorInvalid()
	}
	values := map[string]any{"field": predicate.Field, "operator": predicate.Operator}
	switch predicate.Operator {
	case "isNull", "isNotNull":
		if predicate.HasArgument {
			return nil, behaviorInvalid()
		}
	case "eq", "ne", "lt", "lte", "gt", "gte", "in":
		index := predicate.Argument
		if !predicate.HasArgument || index < 0 || index >= 32 || index >= len(args) {
			return nil, behaviorInvalid()
		}
		if !behaviorQueryOperand(predicate.Operator, args[index]) {
			return nil, behaviorInvalid()
		}
		used[index] = true
		values["value"] = args[index]
	default:
		return nil, behaviorInvalid()
	}
	return values, nil
}

func behaviorQueryOperand(operator string, value BehaviorValue) bool {
	if operator != "in" {
		return behaviorQueryScalar(value)
	}
	values, isList := value.([]any)
	if !isList || len(values) > 100 {
		return false
	}
	for _, item := range values {
		if !behaviorQueryScalar(item) {
			return false
		}
	}
	return true
}

func behaviorQueryOrder(policy BehaviorQueryPolicy) ([]any, error) {
	seen := map[string]bool{}
	order := []any{}
	for _, sort := range policy.OrderBy {
		known := false
		for _, field := range policy.Fields {
			if field == sort.Field {
				known = true
			}
		}
		if !known || seen[sort.Field] || (sort.Direction != "asc" && sort.Direction != "desc") {
			return nil, behaviorInvalid()
		}
		seen[sort.Field] = true
		order = append(order, map[string]any{"field": sort.Field, "direction": sort.Direction})
	}
	return order, nil
}

func behaviorQueryPage(policy BehaviorQueryPolicy, args []BehaviorValue, used map[int]bool) (BehaviorValue, error) {
	orderedById := false
	for _, sort := range policy.OrderBy {
		if sort.Field == policy.Entity.Id {
			orderedById = true
		}
	}
	if !orderedById {
		return nil, behaviorInvalid()
	}
	var raw BehaviorValue = map[string]any{}
	if policy.HasPageArgument {
		index := policy.PageArgument
		if index < 0 || index >= 32 || used[index] || index >= len(args) {
			return nil, behaviorInvalid()
		}
		used[index] = true
		raw = args[index]
	}
	page, err := behaviorPage(raw, &BehaviorPagePolicy{Argument: 0, DefaultSize: policy.DefaultSize, MaxSize: policy.MaxSize, Sort: policy.Fields})
	if err != nil {
		return nil, err
	}
	sort, _ := page.(map[string]any)["sort"].([]any)
	if len(sort) != 0 {
		return nil, behaviorInvalid()
	}
	return page, nil
}

func behaviorQueryScalar(value BehaviorValue) bool {
	switch typed := value.(type) {
	case nil, bool, string:
		return true
	case float64:
		return !math.IsNaN(typed) && !math.IsInf(typed, 0)
	}
	return false
}

func behaviorEntityId(value BehaviorValue) error {
	switch typed := value.(type) {
	case string:
		if typed != "" && len(typed) <= 256 {
			return nil
		}
	case float64:
		if behaviorPropertyNumber(typed, true) {
			return nil
		}
	}
	return behaviorInvalid()
}

func behaviorExchangePath(policy *BehaviorHttpPolicy, args []BehaviorValue) (*BehaviorHttpPolicy, error) {
	path := policy.Path
	for _, entry := range policy.PathVariables {
		if entry.Argument < 0 || entry.Argument >= len(args) {
			return nil, behaviorInvalid()
		}
		text := ""
		switch value := args[entry.Argument].(type) {
		case string:
			text = value
		case bool:
			text = strconv.FormatBool(value)
		case float64:
			text = strconv.FormatFloat(value, 'f', -1, 64)
		case int:
			text = strconv.Itoa(value)
		case int64:
			text = strconv.FormatInt(value, 10)
		default:
			return nil, behaviorInvalid()
		}
		if !behaviorPathValue(text) {
			return nil, behaviorInvalid()
		}
		path = strings.ReplaceAll(path, "{"+entry.Name+"}", text)
	}
	filled := *policy
	filled.Path = path
	return &filled, nil
}

func behaviorPathValue(text string) bool {
	if text == "" || text == "." || text == ".." || len(text) > 256 {
		return false
	}
	for _, character := range text {
		letter := (character >= 'a' && character <= 'z') || (character >= 'A' && character <= 'Z')
		digit := character >= '0' && character <= '9'
		if !letter && !digit && !strings.ContainsRune("._~-", character) {
			return false
		}
	}
	return true
}

func behaviorExchangeRequest(policy *BehaviorHttpPolicy, args []BehaviorValue) (BehaviorValue, error) {
	query := map[string]any{}
	for _, entry := range policy.Query {
		if entry.Argument < 0 || entry.Argument >= len(args) {
			return nil, behaviorInvalid()
		}
		query[entry.Name] = args[entry.Argument]
	}
	headers := map[string]any{}
	for _, entry := range policy.Headers {
		if entry.Argument < 0 || entry.Argument >= len(args) {
			return nil, behaviorInvalid()
		}
		headers[entry.Name] = args[entry.Argument]
	}
	descriptor := map[string]any{"query": query, "headers": headers}
	if policy.HasBody {
		if policy.Body < 0 || policy.Body >= len(args) {
			return nil, behaviorInvalid()
		}
		descriptor["body"] = args[policy.Body]
	}
	return descriptor, nil
}

func behaviorHttpRequest(policy *BehaviorHttpPolicy, input BehaviorValue) (*BehaviorHttpRequest, error) {
	if !behaviorIdentifier(policy.Client) || !behaviorHttpMethod(policy.Method) || !behaviorSafePath(policy.Path) {
		return nil, behaviorInvalid()
	}
	checked, err := behaviorChecked(input, 1048576)
	if err != nil {
		return nil, err
	}
	value, isObject := checked.(map[string]any)
	if !isObject {
		return nil, behaviorInvalid()
	}
	for key := range value {
		if key != "query" && key != "headers" && key != "body" {
			return nil, behaviorInvalid()
		}
	}
	query := map[string]any{}
	if raw, present := value["query"]; present {
		object, isQuery := raw.(map[string]any)
		if !isQuery {
			return nil, behaviorInvalid()
		}
		query = object
	}
	headers := map[string]any{}
	if raw, present := value["headers"]; present {
		object, isHeaders := raw.(map[string]any)
		if !isHeaders {
			return nil, behaviorInvalid()
		}
		headers = object
	}
	body, hasBody := value["body"]
	if (policy.Method == "GET" || policy.Method == "HEAD") && hasBody {
		return nil, behaviorInvalid()
	}
	if !behaviorValidQueryValues(query) || !behaviorValidHeaders(headers) {
		return nil, behaviorInvalid()
	}
	return &BehaviorHttpRequest{
		Method:           policy.Method,
		Path:             policy.Path,
		Query:            query,
		Headers:          headers,
		Body:             body,
		HasBody:          hasBody,
		Redirect:         "error",
		Credentials:      "omit",
		MaxResponseBytes: 1048576,
	}, nil
}

func behaviorValidQueryValues(query map[string]any) bool {
	for _, value := range query {
		switch value.(type) {
		case string, float64, bool:
		default:
			return false
		}
	}
	return true
}

func behaviorValidHeaders(headers map[string]any) bool {
	for key, value := range headers {
		text, isText := value.(string)
		if !isText || !behaviorAllowedHeader(key, text) {
			return false
		}
	}
	return true
}

func behaviorAllowedHeader(key string, value string) bool {
	if key == "" || len(key) > 128 || strings.ContainsAny(value, "\r\n\x00") {
		return false
	}
	for index := 0; index < len(key); index++ {
		current := key[index]
		alphanumeric := (current >= 'a' && current <= 'z') || (current >= 'A' && current <= 'Z') || (current >= '0' && current <= '9')
		if !alphanumeric && !strings.ContainsRune("!#$%&'*+.^_`|~-", rune(current)) {
			return false
		}
	}
	switch strings.ToLower(key) {
	case "host", "connection", "content-length", "transfer-encoding", "cookie":
		return false
	}
	return true
}

func behaviorSafePath(path string) bool {
	if len(path) > 2048 || !strings.HasPrefix(path, "/") || strings.HasPrefix(path, "//") {
		return false
	}
	decoded := []byte{}
	for index := 0; index < len(path); index++ {
		current := path[index]
		if current == '%' {
			if index+2 >= len(path) {
				return false
			}
			value, err := strconv.ParseUint(path[index+1:index+3], 16, 8)
			if err != nil {
				return false
			}
			current = uint8(value)
			index += 2
		}
		if current <= ' ' || strings.ContainsRune("\\?#%", rune(current)) {
			return false
		}
		decoded = append(decoded, current)
	}
	if !utf8.Valid(decoded) {
		return false
	}
	text := string(decoded)
	if strings.HasPrefix(text, "//") || strings.ContainsFunc(text, behaviorUnicodeWhitespace) {
		return false
	}
	for _, part := range strings.Split(text, "/") {
		if part == "." || part == ".." {
			return false
		}
	}
	return true
}

type behaviorCallLog struct {
	mutex sync.Mutex
	keys  []any
}

var behaviorCalls behaviorCallLog

func behaviorCallsRecord(key string) {
	behaviorCalls.mutex.Lock()
	defer behaviorCalls.mutex.Unlock()
	for _, known := range behaviorCalls.keys {
		if known == key {
			return
		}
	}
	behaviorCalls.keys = append(behaviorCalls.keys, key)
}

func behaviorCallsTake() []any {
	behaviorCalls.mutex.Lock()
	defer behaviorCalls.mutex.Unlock()
	keys := behaviorCalls.keys
	behaviorCalls.keys = []any{}
	if keys == nil {
		return []any{}
	}
	return keys
}
