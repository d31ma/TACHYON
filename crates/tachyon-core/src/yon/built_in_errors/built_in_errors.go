
//tachyon:file builtinerrors
package main

import __yonBuiltInJSON "encoding/json"

type BrokenRule struct {
	Field string `json:"field"`
	Rule  string `json:"rule"`
}

type yonBuiltInFields struct {
	Type       string       `json:"type"`
	Reference  string       `json:"reference"`
	Part       string       `json:"part"`
	Field      string       `json:"field"`
	Message    string       `json:"message"`
	LimitBytes int64        `json:"limitBytes"`
	Violations []BrokenRule `json:"violations"`
	Truncated  bool         `json:"truncated"`
}

type NotSignedIn struct {
	Reference string
}

func (failure *NotSignedIn) Error() string { return "NotSignedIn" }

func (failure *NotSignedIn) Status() int { return 401 }

type RoleMissing struct {
	Reference string
}

func (failure *RoleMissing) Error() string { return "RoleMissing" }

func (failure *RoleMissing) Status() int { return 403 }

type IdentityCheckFailed struct {
	Reference string
}

func (failure *IdentityCheckFailed) Error() string { return "IdentityCheckFailed" }

func (failure *IdentityCheckFailed) Status() int { return 500 }

type RequestRejected struct {
	Reference string
	Part      string
	Field     string
	Message   string
}

func (failure *RequestRejected) Error() string { return "RequestRejected" }

func (failure *RequestRejected) Status() int { return 400 }

type BodyTooLarge struct {
	Reference  string
	LimitBytes int64
}

func (failure *BodyTooLarge) Error() string { return "BodyTooLarge" }

func (failure *BodyTooLarge) Status() int { return 413 }

type CheckRejected struct {
	Reference  string
	Violations []BrokenRule
	Truncated  bool
}

func (failure *CheckRejected) Error() string { return "CheckRejected" }

func (failure *CheckRejected) Status() int { return 422 }

type UnexpectedError struct {
	Reference string
}

func (failure *UnexpectedError) Error() string { return "UnexpectedError" }

func (failure *UnexpectedError) Status() int { return 500 }

func __yon_built_in_error__(request *yonRequest) error {
	var fields yonBuiltInFields
	if len(request.fields.Arguments) == 0 {
		return yonNoRoute()
	}
	if __yonBuiltInJSON.Unmarshal(request.fields.Arguments[0], &fields) != nil {
		return yonNoRoute()
	}
	switch fields.Type {
	case "NotSignedIn":
		return &NotSignedIn{Reference: fields.Reference}
	case "RoleMissing":
		return &RoleMissing{Reference: fields.Reference}
	case "IdentityCheckFailed":
		return &IdentityCheckFailed{Reference: fields.Reference}
	case "RequestRejected":
		return &RequestRejected{Reference: fields.Reference, Part: fields.Part, Field: fields.Field, Message: fields.Message}
	case "BodyTooLarge":
		return &BodyTooLarge{Reference: fields.Reference, LimitBytes: fields.LimitBytes}
	case "CheckRejected":
		return &CheckRejected{Reference: fields.Reference, Violations: fields.Violations, Truncated: fields.Truncated}
	case "UnexpectedError":
		return &UnexpectedError{Reference: fields.Reference}
	}
	return yonNoRoute()
}
