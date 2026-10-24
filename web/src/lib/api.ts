// Thin typed wrapper around fetch. The types are generated from the Rust
// structs (`make types`), so request and response shapes can't drift.
import { ApiRequestError } from "@/lib/errors";
import type { ApiError } from "@/types/ApiError";
import type { Credentials } from "@/types/Credentials";
import type { ForgotPassword } from "@/types/ForgotPassword";
import type { NewTodo } from "@/types/NewTodo";
import type { ResetPassword } from "@/types/ResetPassword";
import type { Todo } from "@/types/Todo";
import type { UpdateTodo } from "@/types/UpdateTodo";
import type { User } from "@/types/User";

// The session cookie is HttpOnly, so this code never sees it: fetch sends it
// automatically because the API is on the same origin.
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    const err = (await res.json().catch(() => ({ message: res.statusText }))) as ApiError;
    throw new ApiRequestError(err.message, res.status, res.headers.get("x-request-id"));
  }
  return (res.status === 204 ? undefined : await res.json()) as T;
}

const post = (body: unknown): RequestInit => ({ method: "POST", body: JSON.stringify(body) });

export const api = {
  me: () => request<User>("/auth/me"),
  signup: (input: Credentials) => request<User>("/auth/signup", post(input)),
  login: (input: Credentials) => request<User>("/auth/login", post(input)),
  logout: () => request<void>("/auth/logout", { method: "POST" }),
  forgotPassword: (input: ForgotPassword) => request<void>("/auth/forgot-password", post(input)),
  resetPassword: (input: ResetPassword) => request<User>("/auth/reset-password", post(input)),

  listTodos: () => request<Todo[]>("/todos"),
  createTodo: (input: NewTodo) => request<Todo>("/todos", post(input)),
  updateTodo: (id: number, input: UpdateTodo) =>
    request<Todo>(`/todos/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteTodo: (id: number) => request<void>(`/todos/${id}`, { method: "DELETE" }),
};
