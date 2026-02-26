import { writable, derived } from 'svelte/store';
import type { AuthState, User } from '$lib/types/scada';
import { login as apiLogin, getMe, logout as apiLogout } from '$lib/services/api';

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		user: null,
		token: null,
		isAuthenticated: false
	});

	return {
		subscribe,
		async login(username: string, password: string) {
			const { token, user } = await apiLogin(username, password);
			set({ user, token, isAuthenticated: true });
		},
		async checkAuth() {
			const token = typeof window !== 'undefined' ? localStorage.getItem('scada_token') : null;
			if (!token) {
				set({ user: null, token: null, isAuthenticated: false });
				return false;
			}
			try {
				const user = await getMe();
				set({ user, token, isAuthenticated: true });
				return true;
			} catch {
				apiLogout();
				set({ user: null, token: null, isAuthenticated: false });
				return false;
			}
		},
		logout() {
			apiLogout();
			set({ user: null, token: null, isAuthenticated: false });
		}
	};
}

export const auth = createAuthStore();
export const isAdmin = derived(auth, $auth => $auth.user?.role === 'admin');
export const isOperator = derived(auth, $auth =>
	$auth.user?.role === 'admin' || $auth.user?.role === 'operator'
);
