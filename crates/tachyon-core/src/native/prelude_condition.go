package tac

import (
	"strconv"
	"strings"
)

type conditionOperand struct {
	present bool
	value   any
}

func conditionHolds(condition string, value any, event string) bool {
	matcher, err := parseJSON(condition)
	if err != nil {
		return false
	}
	return matches(matcher, value, event)
}

func matches(matcher any, value any, event string) bool {
	switch nodeText(matcher, "k") {
	case "and":
		return sideMatches(matcher, "l", value, event) && sideMatches(matcher, "r", value, event)
	case "or":
		return sideMatches(matcher, "l", value, event) || sideMatches(matcher, "r", value, event)
	case "not":
		return !sideMatches(matcher, "e", value, event)
	case "truthy":
		node, present := childOf(matcher, "e")
		return present && truthy(operandOf(node, value, event))
	case "cmp":
		return compare(matcher, value, event)
	}
	return false
}

func sideMatches(matcher any, side string, value any, event string) bool {
	node, present := childOf(matcher, side)
	return present && matches(node, value, event)
}

func compare(matcher any, value any, event string) bool {
	leftNode, hasLeft := childOf(matcher, "l")
	rightNode, hasRight := childOf(matcher, "r")
	if !hasLeft || !hasRight {
		return false
	}
	left := operandOf(leftNode, value, event)
	right := operandOf(rightNode, value, event)
	operation := nodeText(matcher, "op")
	if operation == "eq" {
		return identical(left, right)
	}
	if operation == "ne" {
		return !identical(left, right)
	}
	return orders(operation, left, right)
}

func nodeText(node any, key string) string {
	child, _ := childOf(node, key)
	text, _ := child.(string)
	return text
}

func childOf(value any, key string) (any, bool) {
	if record, isRecord := value.(map[string]any); isRecord {
		child, present := record[key]
		return child, present
	}
	list, isList := value.([]any)
	if !isList {
		return nil, false
	}
	index, err := strconv.Atoi(key)
	if err != nil || index < 0 || index >= len(list) {
		return nil, false
	}
	return list[index], true
}

func segmentKey(segment any) (string, bool) {
	if text, isText := segment.(string); isText {
		return text, true
	}
	number, isNumber := segment.(float64)
	if !isNumber || number != float64(int64(number)) {
		return "", false
	}
	return strconv.FormatInt(int64(number), 10), true
}

func operandOf(node any, value any, event string) conditionOperand {
	kind := nodeText(node, "k")
	if kind == "lit" {
		literal, _ := childOf(node, "v")
		return conditionOperand{present: true, value: literal}
	}
	if kind == "event" {
		return conditionOperand{present: true, value: event}
	}
	current := value
	path, hasPath := childOf(node, "path")
	segments, isList := path.([]any)
	if !hasPath || !isList {
		return conditionOperand{present: true, value: current}
	}
	for _, segment := range segments {
		key, isKey := segmentKey(segment)
		if !isKey {
			return conditionOperand{}
		}
		next, present := childOf(current, key)
		if !present {
			return conditionOperand{}
		}
		current = next
	}
	return conditionOperand{present: true, value: current}
}

func numberOperand(operand conditionOperand) (float64, bool) {
	number, isNumber := operand.value.(float64)
	return number, operand.present && isNumber
}

func truthy(operand conditionOperand) bool {
	if !operand.present || operand.value == nil {
		return false
	}
	switch value := operand.value.(type) {
	case bool:
		return value
	case string:
		return value != ""
	case float64:
		return value != 0
	}
	return true
}

func identical(left conditionOperand, right conditionOperand) bool {
	if !left.present || !right.present {
		return !left.present && !right.present
	}
	return sameValue(left.value, right.value)
}

func sameValue(left any, right any) bool {
	switch leftValue := left.(type) {
	case nil:
		return right == nil
	case bool:
		rightValue, same := right.(bool)
		return same && leftValue == rightValue
	case string:
		rightValue, same := right.(string)
		return same && leftValue == rightValue
	case float64:
		rightValue, same := right.(float64)
		return same && leftValue == rightValue
	case []any:
		rightValue, same := right.([]any)
		return same && sameLists(leftValue, rightValue)
	case map[string]any:
		rightValue, same := right.(map[string]any)
		return same && sameRecords(leftValue, rightValue)
	}
	return false
}

func sameLists(left []any, right []any) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if !sameValue(left[index], right[index]) {
			return false
		}
	}
	return true
}

func sameRecords(left map[string]any, right map[string]any) bool {
	if len(left) != len(right) {
		return false
	}
	for key, value := range left {
		held, present := right[key]
		if !present || !sameValue(value, held) {
			return false
		}
	}
	return true
}

func orders(operation string, left conditionOperand, right conditionOperand) bool {
	comparison, comparable := compareOperands(left, right)
	if !comparable {
		return false
	}
	switch operation {
	case "lt":
		return comparison < 0
	case "le":
		return comparison <= 0
	case "gt":
		return comparison > 0
	}
	return comparison >= 0
}

func compareOperands(left conditionOperand, right conditionOperand) (int, bool) {
	leftNumber, leftIsNumber := numberOperand(left)
	rightNumber, rightIsNumber := numberOperand(right)
	if leftIsNumber && rightIsNumber {
		if leftNumber < rightNumber {
			return -1, true
		}
		if leftNumber > rightNumber {
			return 1, true
		}
		return 0, true
	}
	leftText, leftIsText := left.value.(string)
	rightText, rightIsText := right.value.(string)
	if left.present && right.present && leftIsText && rightIsText {
		return strings.Compare(leftText, rightText), true
	}
	return 0, false
}
