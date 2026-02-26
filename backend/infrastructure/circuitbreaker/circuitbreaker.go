package circuitbreaker

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"
)

// State represents the circuit breaker state
type State int

const (
	StateClosed   State = iota // Normal operation — requests pass through
	StateOpen                  // Failing — requests are rejected immediately
	StateHalfOpen              // Recovery — limited requests allowed to test
)

func (s State) String() string {
	switch s {
	case StateClosed:
		return "closed"
	case StateOpen:
		return "open"
	case StateHalfOpen:
		return "half-open"
	default:
		return "unknown"
	}
}

var (
	ErrCircuitOpen = errors.New("circuit breaker is open")
	ErrTooManyRequests = errors.New("circuit breaker is half-open: too many requests")
)

// Config for circuit breaker tuning
type Config struct {
	Name             string
	MaxFailures      int           // Failures before opening circuit
	ResetTimeout     time.Duration // Time to wait before half-open
	HalfOpenMaxCalls int           // Max concurrent calls in half-open state
	SuccessThreshold int           // Successes needed to close from half-open
	OnStateChange    func(name string, from, to State)
}

func DefaultConfig(name string) Config {
	return Config{
		Name:             name,
		MaxFailures:      5,
		ResetTimeout:     30 * time.Second,
		HalfOpenMaxCalls: 3,
		SuccessThreshold: 2,
	}
}

// CircuitBreaker implements the circuit breaker pattern
type CircuitBreaker struct {
	config Config
	mu     sync.Mutex

	state       State
	failures    int
	successes   int
	lastFailure time.Time
	halfOpenCalls int

	// Metrics
	totalCalls   int64
	totalSuccess int64
	totalFailure int64
	totalReject  int64
}

func New(config Config) *CircuitBreaker {
	return &CircuitBreaker{
		config: config,
		state:  StateClosed,
	}
}

// Execute runs the given function with circuit breaker protection
func (cb *CircuitBreaker) Execute(ctx context.Context, fn func(ctx context.Context) error) error {
	if err := cb.beforeCall(); err != nil {
		return err
	}

	err := fn(ctx)

	cb.afterCall(err)
	return err
}

// ExecuteWithResult runs a function that returns a value with circuit breaker protection
func ExecuteWithResult[T any](cb *CircuitBreaker, ctx context.Context, fn func(ctx context.Context) (T, error)) (T, error) {
	var zero T
	if err := cb.beforeCall(); err != nil {
		return zero, err
	}

	result, err := fn(ctx)
	cb.afterCall(err)
	return result, err
}

func (cb *CircuitBreaker) beforeCall() error {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	cb.totalCalls++

	switch cb.state {
	case StateClosed:
		return nil

	case StateOpen:
		// Check if reset timeout has elapsed
		if time.Since(cb.lastFailure) > cb.config.ResetTimeout {
			cb.transitionTo(StateHalfOpen)
			cb.halfOpenCalls = 1
			return nil
		}
		cb.totalReject++
		return ErrCircuitOpen

	case StateHalfOpen:
		if cb.halfOpenCalls >= cb.config.HalfOpenMaxCalls {
			cb.totalReject++
			return ErrTooManyRequests
		}
		cb.halfOpenCalls++
		return nil
	}

	return nil
}

func (cb *CircuitBreaker) afterCall(err error) {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	if err != nil {
		cb.onFailure()
	} else {
		cb.onSuccess()
	}
}

func (cb *CircuitBreaker) onSuccess() {
	cb.totalSuccess++

	switch cb.state {
	case StateClosed:
		cb.failures = 0

	case StateHalfOpen:
		cb.successes++
		if cb.successes >= cb.config.SuccessThreshold {
			cb.transitionTo(StateClosed)
		}
	}
}

func (cb *CircuitBreaker) onFailure() {
	cb.totalFailure++
	cb.lastFailure = time.Now()

	switch cb.state {
	case StateClosed:
		cb.failures++
		if cb.failures >= cb.config.MaxFailures {
			cb.transitionTo(StateOpen)
		}

	case StateHalfOpen:
		// Any failure in half-open immediately re-opens
		cb.transitionTo(StateOpen)
	}
}

func (cb *CircuitBreaker) transitionTo(newState State) {
	if cb.state == newState {
		return
	}

	oldState := cb.state
	cb.state = newState
	cb.failures = 0
	cb.successes = 0
	cb.halfOpenCalls = 0

	if cb.config.OnStateChange != nil {
		go cb.config.OnStateChange(cb.config.Name, oldState, newState)
	}
}

// State returns the current circuit breaker state
func (cb *CircuitBreaker) State() State {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	return cb.state
}

// Metrics returns circuit breaker statistics
func (cb *CircuitBreaker) Metrics() Metrics {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	return Metrics{
		Name:         cb.config.Name,
		State:        cb.state.String(),
		TotalCalls:   cb.totalCalls,
		TotalSuccess: cb.totalSuccess,
		TotalFailure: cb.totalFailure,
		TotalReject:  cb.totalReject,
		Failures:     cb.failures,
		LastFailure:  cb.lastFailure,
	}
}

type Metrics struct {
	Name         string    `json:"name"`
	State        string    `json:"state"`
	TotalCalls   int64     `json:"total_calls"`
	TotalSuccess int64     `json:"total_success"`
	TotalFailure int64     `json:"total_failure"`
	TotalReject  int64     `json:"total_reject"`
	Failures     int       `json:"current_failures"`
	LastFailure  time.Time `json:"last_failure"`
}

func (m Metrics) String() string {
	return fmt.Sprintf("[%s] state=%s calls=%d ok=%d fail=%d reject=%d",
		m.Name, m.State, m.TotalCalls, m.TotalSuccess, m.TotalFailure, m.TotalReject)
}
