package errors

import "fmt"

// Domain errors — these are technology-agnostic

type NotFoundError struct {
	Entity string
	ID     string
}

func (e *NotFoundError) Error() string {
	return fmt.Sprintf("%s not found: %s", e.Entity, e.ID)
}

type ValidationError struct {
	Field   string
	Message string
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("validation error on %s: %s", e.Field, e.Message)
}

type AuthenticationError struct {
	Message string
}

func (e *AuthenticationError) Error() string {
	return fmt.Sprintf("authentication failed: %s", e.Message)
}

type AuthorizationError struct {
	Role   string
	Action string
}

func (e *AuthorizationError) Error() string {
	return fmt.Sprintf("role %q is not authorized to %s", e.Role, e.Action)
}

type ConflictError struct {
	Entity  string
	Message string
}

func (e *ConflictError) Error() string {
	return fmt.Sprintf("%s conflict: %s", e.Entity, e.Message)
}

type ServiceUnavailableError struct {
	Service string
	Err     error
}

func (e *ServiceUnavailableError) Error() string {
	return fmt.Sprintf("service %s unavailable: %v", e.Service, e.Err)
}

func (e *ServiceUnavailableError) Unwrap() error { return e.Err }

// Helper constructors

func NewNotFound(entity, id string) error {
	return &NotFoundError{Entity: entity, ID: id}
}

func NewValidation(field, message string) error {
	return &ValidationError{Field: field, Message: message}
}

func NewAuthFailed(msg string) error {
	return &AuthenticationError{Message: msg}
}

func NewForbidden(role, action string) error {
	return &AuthorizationError{Role: role, Action: action}
}

func NewUnavailable(service string, err error) error {
	return &ServiceUnavailableError{Service: service, Err: err}
}

// Type checks

func IsNotFound(err error) bool {
	_, ok := err.(*NotFoundError)
	return ok
}

func IsValidation(err error) bool {
	_, ok := err.(*ValidationError)
	return ok
}

func IsAuthError(err error) bool {
	_, ok := err.(*AuthenticationError)
	return ok
}

func IsUnavailable(err error) bool {
	_, ok := err.(*ServiceUnavailableError)
	return ok
}
