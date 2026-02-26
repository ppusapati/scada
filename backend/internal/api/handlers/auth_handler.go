package handlers

import (
	"encoding/json"
	"net/http"

	"scada-system/internal/api/middleware"
	"scada-system/internal/services"
)

type AuthHandler struct {
	auth    *services.AuthService
	authMW  *middleware.AuthMiddleware
	expHours int
}

func NewAuthHandler(auth *services.AuthService, authMW *middleware.AuthMiddleware, expHours int) *AuthHandler {
	return &AuthHandler{auth: auth, authMW: authMW, expHours: expHours}
}

func (h *AuthHandler) Login(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Username string `json:"username"`
		Password string `json:"password"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request body"})
		return
	}

	user, err := h.auth.Login(r.Context(), req.Username, req.Password)
	if err != nil {
		writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "invalid credentials"})
		return
	}

	token, err := h.authMW.GenerateToken(user.ID, user.Username, user.Role, h.expHours)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to generate token"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"token": token,
		"user":  user,
	})
}

func (h *AuthHandler) Register(w http.ResponseWriter, r *http.Request) {
	// Only admins can register new users
	role := middleware.GetUserRole(r.Context())
	if role != "admin" {
		writeJSON(w, http.StatusForbidden, map[string]string{"error": "admin only"})
		return
	}

	var req struct {
		Username string `json:"username"`
		Email    string `json:"email"`
		Password string `json:"password"`
		Role     string `json:"role"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request body"})
		return
	}

	if req.Role == "" {
		req.Role = "viewer"
	}

	user, err := h.auth.CreateUser(r.Context(), req.Username, req.Email, req.Password, req.Role)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusCreated, user)
}

func (h *AuthHandler) Me(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	user, err := h.auth.GetUser(r.Context(), userID)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "user not found"})
		return
	}
	writeJSON(w, http.StatusOK, user)
}
