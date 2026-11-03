package tac

import (
	"errors"
	"math"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"unicode/utf16"
	"unicode/utf8"
)

type jsonParser struct {
	text     string
	position int
	depth    int
}

var invalidJSON = errors.New("invalid JSON")
var notJSON = errors.New("value is not JSON")

func parseJSON(text string) (any, error) {
	parser := jsonParser{text: text}
	value, err := parser.value()
	if err != nil {
		return nil, err
	}
	parser.skipSpace()
	if parser.position != len(parser.text) {
		return nil, invalidJSON
	}
	return value, nil
}

func (parser *jsonParser) skipSpace() {
	for parser.position < len(parser.text) {
		switch parser.text[parser.position] {
		case ' ', '\t', '\n', '\r':
			parser.position++
		default:
			return
		}
	}
}

func (parser *jsonParser) value() (any, error) {
	parser.skipSpace()
	if parser.position >= len(parser.text) {
		return nil, invalidJSON
	}
	switch parser.text[parser.position] {
	case '{':
		return parser.object()
	case '[':
		return parser.list()
	case '"':
		return parser.quoted()
	case 't':
		return parser.word("true", true)
	case 'f':
		return parser.word("false", false)
	case 'n':
		return parser.word("null", nil)
	}
	return parser.number()
}

func (parser *jsonParser) word(word string, value any) (any, error) {
	if !strings.HasPrefix(parser.text[parser.position:], word) {
		return nil, invalidJSON
	}
	parser.position += len(word)
	return value, nil
}

func (parser *jsonParser) number() (any, error) {
	start := parser.position
	for parser.position < len(parser.text) && strings.IndexByte("+-0123456789.eE", parser.text[parser.position]) >= 0 {
		parser.position++
	}
	number, err := strconv.ParseFloat(parser.text[start:parser.position], 64)
	if err != nil {
		return nil, invalidJSON
	}
	return number, nil
}

func (parser *jsonParser) enter() error {
	parser.depth++
	if parser.depth > 64 {
		return invalidJSON
	}
	parser.position++
	return nil
}

func (parser *jsonParser) list() (any, error) {
	if err := parser.enter(); err != nil {
		return nil, err
	}
	items := []any{}
	parser.skipSpace()
	if parser.next(']') {
		parser.depth--
		return items, nil
	}
	for {
		item, err := parser.value()
		if err != nil {
			return nil, err
		}
		items = append(items, item)
		parser.skipSpace()
		if parser.next(']') {
			parser.depth--
			return items, nil
		}
		if !parser.next(',') {
			return nil, invalidJSON
		}
	}
}

func (parser *jsonParser) object() (any, error) {
	if err := parser.enter(); err != nil {
		return nil, err
	}
	record := map[string]any{}
	parser.skipSpace()
	if parser.next('}') {
		parser.depth--
		return record, nil
	}
	for {
		parser.skipSpace()
		if parser.position >= len(parser.text) || parser.text[parser.position] != '"' {
			return nil, invalidJSON
		}
		key, err := parser.quoted()
		if err != nil {
			return nil, err
		}
		parser.skipSpace()
		if !parser.next(':') {
			return nil, invalidJSON
		}
		value, err := parser.value()
		if err != nil {
			return nil, err
		}
		record[key.(string)] = value
		parser.skipSpace()
		if parser.next('}') {
			parser.depth--
			return record, nil
		}
		if !parser.next(',') {
			return nil, invalidJSON
		}
	}
}

func (parser *jsonParser) next(character byte) bool {
	if parser.position < len(parser.text) && parser.text[parser.position] == character {
		parser.position++
		return true
	}
	return false
}

func (parser *jsonParser) quoted() (any, error) {
	parser.position++
	var builder strings.Builder
	for parser.position < len(parser.text) {
		character := parser.text[parser.position]
		parser.position++
		if character == '"' {
			return builder.String(), nil
		}
		if character != '\\' {
			builder.WriteByte(character)
			continue
		}
		if err := parser.escape(&builder); err != nil {
			return nil, err
		}
	}
	return nil, invalidJSON
}

func (parser *jsonParser) escape(builder *strings.Builder) error {
	if parser.position >= len(parser.text) {
		return invalidJSON
	}
	character := parser.text[parser.position]
	parser.position++
	switch character {
	case '"', '\\', '/':
		builder.WriteByte(character)
	case 'b':
		builder.WriteByte('\b')
	case 'f':
		builder.WriteByte('\f')
	case 'n':
		builder.WriteByte('\n')
	case 'r':
		builder.WriteByte('\r')
	case 't':
		builder.WriteByte('\t')
	case 'u':
		return parser.unicode(builder)
	default:
		return invalidJSON
	}
	return nil
}

func (parser *jsonParser) unicode(builder *strings.Builder) error {
	first, err := parser.hexadecimal()
	if err != nil {
		return err
	}
	if !utf16.IsSurrogate(first) {
		builder.WriteRune(first)
		return nil
	}
	if !strings.HasPrefix(parser.text[parser.position:], "\\u") {
		builder.WriteRune(utf8.RuneError)
		return nil
	}
	parser.position += 2
	second, err := parser.hexadecimal()
	if err != nil {
		return err
	}
	builder.WriteRune(utf16.DecodeRune(first, second))
	return nil
}

func (parser *jsonParser) hexadecimal() (rune, error) {
	if parser.position+4 > len(parser.text) {
		return 0, invalidJSON
	}
	number, err := strconv.ParseUint(parser.text[parser.position:parser.position+4], 16, 32)
	if err != nil {
		return 0, invalidJSON
	}
	parser.position += 4
	return rune(number), nil
}

func encodeJSON(value any) (string, error) {
	var builder strings.Builder
	if err := writeValue(&builder, reflect.ValueOf(value), 0); err != nil {
		return "", err
	}
	return builder.String(), nil
}

func writeValue(builder *strings.Builder, value reflect.Value, depth int) error {
	if depth > 64 {
		return notJSON
	}
	switch value.Kind() {
	case reflect.Invalid:
		builder.WriteString("null")
	case reflect.Bool:
		builder.WriteString(strconv.FormatBool(value.Bool()))
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		builder.WriteString(strconv.FormatInt(value.Int(), 10))
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		builder.WriteString(strconv.FormatUint(value.Uint(), 10))
	case reflect.Float32, reflect.Float64:
		return writeNumber(builder, value.Float())
	case reflect.String:
		builder.WriteString(quote(value.String()))
	case reflect.Slice, reflect.Array:
		return writeList(builder, value, depth)
	case reflect.Map:
		return writeMap(builder, value, depth)
	case reflect.Struct:
		return writeStruct(builder, value, depth)
	case reflect.Pointer, reflect.Interface:
		if value.IsNil() {
			builder.WriteString("null")
			return nil
		}
		return writeValue(builder, value.Elem(), depth+1)
	default:
		return notJSON
	}
	return nil
}

func writeNumber(builder *strings.Builder, number float64) error {
	if math.IsNaN(number) || math.IsInf(number, 0) {
		return notJSON
	}
	builder.WriteString(strconv.FormatFloat(number, 'g', -1, 64))
	return nil
}

func writeList(builder *strings.Builder, value reflect.Value, depth int) error {
	builder.WriteByte('[')
	for index := 0; index < value.Len(); index++ {
		if index > 0 {
			builder.WriteByte(',')
		}
		if err := writeValue(builder, value.Index(index), depth+1); err != nil {
			return err
		}
	}
	builder.WriteByte(']')
	return nil
}

func writeMap(builder *strings.Builder, value reflect.Value, depth int) error {
	if value.Type().Key().Kind() != reflect.String {
		return notJSON
	}
	keys := []string{}
	for _, key := range value.MapKeys() {
		keys = append(keys, key.String())
	}
	sort.Strings(keys)
	builder.WriteByte('{')
	for index, key := range keys {
		if index > 0 {
			builder.WriteByte(',')
		}
		builder.WriteString(quote(key))
		builder.WriteByte(':')
		entry := value.MapIndex(reflect.ValueOf(key).Convert(value.Type().Key()))
		if err := writeValue(builder, entry, depth+1); err != nil {
			return err
		}
	}
	builder.WriteByte('}')
	return nil
}

func writeStruct(builder *strings.Builder, value reflect.Value, depth int) error {
	builder.WriteByte('{')
	written := 0
	for index := 0; index < value.NumField(); index++ {
		key, exposed := fieldKey(value.Type().Field(index))
		if !exposed {
			continue
		}
		if written > 0 {
			builder.WriteByte(',')
		}
		written++
		builder.WriteString(quote(key))
		builder.WriteByte(':')
		if err := writeValue(builder, value.Field(index), depth+1); err != nil {
			return err
		}
	}
	builder.WriteByte('}')
	return nil
}

func fieldKey(field reflect.StructField) (string, bool) {
	if !field.IsExported() {
		return "", false
	}
	tag, _, _ := strings.Cut(field.Tag.Get("json"), ",")
	if tag == "-" {
		return "", false
	}
	if tag != "" {
		return tag, true
	}
	return strings.ToLower(field.Name[:1]) + field.Name[1:], true
}

func quote(text string) string {
	var builder strings.Builder
	builder.WriteByte('"')
	for _, character := range text {
		switch {
		case character == '"':
			builder.WriteString("\\\"")
		case character == '\\':
			builder.WriteString("\\\\")
		case character == '\n':
			builder.WriteString("\\n")
		case character == '\r':
			builder.WriteString("\\r")
		case character == '\t':
			builder.WriteString("\\t")
		case character < 0x20:
			builder.WriteString("\\u00")
			builder.WriteByte("0123456789abcdef"[character>>4])
			builder.WriteByte("0123456789abcdef"[character&0xf])
		default:
			builder.WriteRune(character)
		}
	}
	builder.WriteByte('"')
	return builder.String()
}

func Argument(arguments []any, index int, target any) error {
	if index >= len(arguments) {
		return nil
	}
	return Assign(target, arguments[index])
}

func Assign(target any, value any) error {
	return setValue(reflect.ValueOf(target).Elem(), value)
}

func setValue(target reflect.Value, value any) error {
	switch target.Kind() {
	case reflect.Interface:
		return setInterface(target, value)
	case reflect.String:
		target.SetString(textOf(value))
	case reflect.Bool:
		target.SetBool(flagOf(value))
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		target.SetInt(int64(numberOf(value)))
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		target.SetUint(uint64(math.Max(numberOf(value), 0)))
	case reflect.Float32, reflect.Float64:
		target.SetFloat(numberOf(value))
	case reflect.Slice:
		return setSlice(target, value)
	case reflect.Map:
		return setMap(target, value)
	case reflect.Struct:
		return setStruct(target, value)
	case reflect.Pointer:
		return setPointer(target, value)
	default:
		return notJSON
	}
	return nil
}

func setInterface(target reflect.Value, value any) error {
	if value == nil {
		target.SetZero()
		return nil
	}
	given := reflect.ValueOf(value)
	if !given.Type().AssignableTo(target.Type()) {
		return notJSON
	}
	target.Set(given)
	return nil
}

func setSlice(target reflect.Value, value any) error {
	if value == nil {
		target.SetZero()
		return nil
	}
	items, isList := value.([]any)
	if !isList {
		return notJSON
	}
	slice := reflect.MakeSlice(target.Type(), len(items), len(items))
	for index, item := range items {
		if err := setValue(slice.Index(index), item); err != nil {
			return err
		}
	}
	target.Set(slice)
	return nil
}

func setMap(target reflect.Value, value any) error {
	if value == nil {
		target.SetZero()
		return nil
	}
	record, isRecord := value.(map[string]any)
	if !isRecord || target.Type().Key().Kind() != reflect.String {
		return notJSON
	}
	created := reflect.MakeMapWithSize(target.Type(), len(record))
	for key, item := range record {
		entry := reflect.New(target.Type().Elem()).Elem()
		if err := setValue(entry, item); err != nil {
			return err
		}
		created.SetMapIndex(reflect.ValueOf(key).Convert(target.Type().Key()), entry)
	}
	target.Set(created)
	return nil
}

func setStruct(target reflect.Value, value any) error {
	if value == nil {
		target.SetZero()
		return nil
	}
	record, isRecord := value.(map[string]any)
	if !isRecord {
		return notJSON
	}
	for index := 0; index < target.NumField(); index++ {
		key, exposed := fieldKey(target.Type().Field(index))
		item, present := record[key]
		if !exposed || !present {
			continue
		}
		if err := setValue(target.Field(index), item); err != nil {
			return err
		}
	}
	return nil
}

func setPointer(target reflect.Value, value any) error {
	if value == nil {
		target.SetZero()
		return nil
	}
	created := reflect.New(target.Type().Elem())
	if err := setValue(created.Elem(), value); err != nil {
		return err
	}
	target.Set(created)
	return nil
}

func textOf(value any) string {
	switch given := value.(type) {
	case string:
		return given
	case float64:
		return strconv.FormatFloat(given, 'g', -1, 64)
	case bool:
		return strconv.FormatBool(given)
	}
	return ""
}

func numberOf(value any) float64 {
	switch given := value.(type) {
	case float64:
		return given
	case bool:
		if given {
			return 1
		}
	case string:
		number, err := strconv.ParseFloat(strings.TrimSpace(given), 64)
		if err == nil {
			return number
		}
	}
	return 0
}

func flagOf(value any) bool {
	switch given := value.(type) {
	case bool:
		return given
	case float64:
		return given != 0
	case string:
		return given != ""
	case []any:
		return len(given) > 0
	case map[string]any:
		return len(given) > 0
	}
	return false
}
