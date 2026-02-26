package services

import (
	"context"
	"errors"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"golang.org/x/crypto/bcrypt"

	"scada-system/internal/db/models"
)

type AuthService struct {
	pool *pgxpool.Pool
}

func NewAuthService(pool *pgxpool.Pool) *AuthService {
	return &AuthService{pool: pool}
}

func (s *AuthService) Login(ctx context.Context, username, password string) (*models.User, error) {
	var user models.User
	err := s.pool.QueryRow(ctx,
		`SELECT id, username, email, password_hash, role, active, created_at, last_login
		FROM users WHERE username = $1 AND active = true`, username).
		Scan(&user.ID, &user.Username, &user.Email, &user.PasswordHash,
			&user.Role, &user.Active, &user.CreatedAt, &user.LastLogin)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, errors.New("invalid credentials")
		}
		return nil, err
	}

	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(password)); err != nil {
		return nil, errors.New("invalid credentials")
	}

	// Update last login
	now := time.Now()
	s.pool.Exec(ctx, `UPDATE users SET last_login = $1 WHERE id = $2`, now, user.ID)
	user.LastLogin = &now

	return &user, nil
}

func (s *AuthService) CreateUser(ctx context.Context, username, email, password, role string) (*models.User, error) {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	var user models.User
	err = s.pool.QueryRow(ctx,
		`INSERT INTO users (username, email, password_hash, role)
		VALUES ($1, $2, $3, $4) RETURNING id, username, email, role, active, created_at`,
		username, email, string(hash), role).
		Scan(&user.ID, &user.Username, &user.Email, &user.Role, &user.Active, &user.CreatedAt)
	if err != nil {
		return nil, err
	}

	return &user, nil
}

func (s *AuthService) GetUser(ctx context.Context, id string) (*models.User, error) {
	var user models.User
	err := s.pool.QueryRow(ctx,
		`SELECT id, username, email, role, active, created_at, last_login
		FROM users WHERE id = $1`, id).
		Scan(&user.ID, &user.Username, &user.Email, &user.Role, &user.Active, &user.CreatedAt, &user.LastLogin)
	if err != nil {
		return nil, err
	}
	return &user, nil
}

func (s *AuthService) ListUsers(ctx context.Context) ([]models.User, error) {
	rows, err := s.pool.Query(ctx,
		`SELECT id, username, email, role, active, created_at, last_login FROM users ORDER BY username`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var users []models.User
	for rows.Next() {
		var u models.User
		if err := rows.Scan(&u.ID, &u.Username, &u.Email, &u.Role, &u.Active, &u.CreatedAt, &u.LastLogin); err != nil {
			return nil, err
		}
		users = append(users, u)
	}
	return users, nil
}
