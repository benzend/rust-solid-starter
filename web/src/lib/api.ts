// Thin typed wrapper around fetch. The types are generated from the Rust
// structs (`make types`), so request and response shapes can't drift.
import type { ApiError } from "@/types/ApiError";
import type { NewTodo } from "@/types/NewTodo";
import type { Todo } from "@/types/Todo";
import type { UpdateTodo } from "@/types/UpdateTodo";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    const err = (await res.json().catch(() => ({ message: res.statusText }))) as ApiError;
    throw new Error(err.message);
  }
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export const api = {
  listTodos: () => request<Todo[]>("/todos"),
  createTodo: (input: NewTodo) =>
    request<Todo>("/todos", { method: "POST", body: JSON.stringify(input) }),
  updateTodo: (id: number, input: UpdateTodo) =>
    request<Todo>(`/todos/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteTodo: (id: number) => request<void>(`/todos/${id}`, { method: "DELETE" }),
};
