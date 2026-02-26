package grpcserver

import (
	"context"
	"fmt"
	"strings"
	"time"

	"connectrpc.com/connect"
	"github.com/golang-jwt/jwt/v5"
)

type contextKey string

const (
	userIDKey   contextKey = "user_id"
	userRoleKey contextKey = "user_role"
)

// LoggingInterceptor logs all RPC calls with duration
func LoggingInterceptor() connect.UnaryInterceptorFunc {
	return func(next connect.UnaryFunc) connect.UnaryFunc {
		return func(ctx context.Context, req connect.AnyRequest) (connect.AnyResponse, error) {
			start := time.Now()
			resp, err := next(ctx, req)
			duration := time.Since(start)

			status := "OK"
			if err != nil {
				status = connect.CodeOf(err).String()
			}

			fmt.Printf("[gRPC] %s %s (%s) %s\n",
				req.Spec().Procedure,
				status,
				duration.Round(time.Microsecond),
				req.Peer().Addr,
			)
			return resp, err
		}
	}
}

// AuthInterceptor validates JWT tokens from the Authorization header
func AuthInterceptor(jwtSecret string, publicProcedures map[string]bool) connect.UnaryInterceptorFunc {
	return func(next connect.UnaryFunc) connect.UnaryFunc {
		return func(ctx context.Context, req connect.AnyRequest) (connect.AnyResponse, error) {
			procedure := req.Spec().Procedure

			// Skip auth for public endpoints
			if publicProcedures[procedure] {
				return next(ctx, req)
			}

			authHeader := req.Header().Get("Authorization")
			if authHeader == "" {
				return nil, connect.NewError(connect.CodeUnauthenticated, fmt.Errorf("missing authorization header"))
			}

			parts := strings.SplitN(authHeader, " ", 2)
			if len(parts) != 2 || parts[0] != "Bearer" {
				return nil, connect.NewError(connect.CodeUnauthenticated, fmt.Errorf("invalid authorization format"))
			}

			claims := &jwtClaims{}
			token, err := jwt.ParseWithClaims(parts[1], claims, func(token *jwt.Token) (interface{}, error) {
				return []byte(jwtSecret), nil
			})
			if err != nil || !token.Valid {
				return nil, connect.NewError(connect.CodeUnauthenticated, fmt.Errorf("invalid or expired token"))
			}

			// Inject user info into context
			ctx = context.WithValue(ctx, userIDKey, claims.UserID)
			ctx = context.WithValue(ctx, userRoleKey, claims.Role)

			return next(ctx, req)
		}
	}
}

// RoleInterceptor enforces RBAC on control operations
func RoleInterceptor(controlProcedures map[string]bool) connect.UnaryInterceptorFunc {
	return func(next connect.UnaryFunc) connect.UnaryFunc {
		return func(ctx context.Context, req connect.AnyRequest) (connect.AnyResponse, error) {
			procedure := req.Spec().Procedure

			if controlProcedures[procedure] {
				role, _ := ctx.Value(userRoleKey).(string)
				if role != "admin" && role != "operator" {
					return nil, connect.NewError(connect.CodePermissionDenied, fmt.Errorf("insufficient permissions: requires operator or admin role"))
				}
			}

			return next(ctx, req)
		}
	}
}

// RecoveryInterceptor catches panics and returns Internal errors
func RecoveryInterceptor() connect.UnaryInterceptorFunc {
	return func(next connect.UnaryFunc) connect.UnaryFunc {
		return func(ctx context.Context, req connect.AnyRequest) (resp connect.AnyResponse, err error) {
			defer func() {
				if r := recover(); r != nil {
					err = connect.NewError(connect.CodeInternal, fmt.Errorf("internal server error"))
					fmt.Printf("[gRPC] PANIC in %s: %v\n", req.Spec().Procedure, r)
				}
			}()
			return next(ctx, req)
		}
	}
}

type jwtClaims struct {
	UserID   string `json:"user_id"`
	Username string `json:"username"`
	Role     string `json:"role"`
	jwt.RegisteredClaims
}
