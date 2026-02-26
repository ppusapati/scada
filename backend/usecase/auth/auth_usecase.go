package auth

import (
	"context"
	"time"

	"golang.org/x/crypto/bcrypt"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
)

// TokenGenerator abstracts JWT token creation
type TokenGenerator interface {
	GenerateToken(userID, username string, role entity.UserRole, expHours int) (string, error)
}

type UseCase struct {
	userRepo  repository.UserRepository
	tokenGen  TokenGenerator
	expHours  int
}

func NewUseCase(userRepo repository.UserRepository, tokenGen TokenGenerator, expHours int) *UseCase {
	return &UseCase{userRepo: userRepo, tokenGen: tokenGen, expHours: expHours}
}

type LoginResult struct {
	Token string
	User  *entity.User
}

func (uc *UseCase) Login(ctx context.Context, username, password string) (*LoginResult, error) {
	if username == "" || password == "" {
		return nil, domainerr.NewValidation("credentials", "username and password required")
	}

	user, err := uc.userRepo.FindByUsername(ctx, username)
	if err != nil {
		return nil, domainerr.NewAuthFailed("invalid credentials")
	}

	if !user.Active {
		return nil, domainerr.NewAuthFailed("account disabled")
	}

	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(password)); err != nil {
		return nil, domainerr.NewAuthFailed("invalid credentials")
	}

	token, err := uc.tokenGen.GenerateToken(user.ID, user.Username, user.Role, uc.expHours)
	if err != nil {
		return nil, err
	}

	uc.userRepo.UpdateLastLogin(ctx, user.ID)
	now := time.Now()
	user.LastLogin = &now

	return &LoginResult{Token: token, User: user}, nil
}

func (uc *UseCase) RegisterUser(ctx context.Context, username, email, password string, role entity.UserRole, callerRole entity.UserRole) (*entity.User, error) {
	if !callerRole.IsAdmin() {
		return nil, domainerr.NewForbidden(string(callerRole), "register users")
	}

	if username == "" || email == "" || password == "" {
		return nil, domainerr.NewValidation("fields", "username, email, and password required")
	}

	if len(password) < 8 {
		return nil, domainerr.NewValidation("password", "must be at least 8 characters")
	}

	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	user := &entity.User{
		Username:     username,
		Email:        email,
		PasswordHash: string(hash),
		Role:         role,
		Active:       true,
	}

	if err := uc.userRepo.Create(ctx, user); err != nil {
		return nil, err
	}

	return user, nil
}

func (uc *UseCase) GetUser(ctx context.Context, id string) (*entity.User, error) {
	user, err := uc.userRepo.FindByID(ctx, id)
	if err != nil {
		return nil, domainerr.NewNotFound("User", id)
	}
	return user, nil
}
