import Plus from "lucide-solid/icons/plus";
import Trash from "lucide-solid/icons/trash";
import { createResource, createSignal, For, Show } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import type { Todo } from "@/types/Todo";

export default function Home() {
  // createResource fetches once; only the parts of the page that read it update.
  const [todos, { mutate }] = createResource(api.listTodos);
  const [title, setTitle] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  const run = async (action: () => Promise<void>) => {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError((err as Error).message);
    }
  };

  const add = (e: SubmitEvent) => {
    e.preventDefault();
    run(async () => {
      const todo = await api.createTodo({ title: title() });
      mutate((prev) => [...(prev ?? []), todo]);
      setTitle("");
    });
  };

  const toggle = (todo: Todo) =>
    run(async () => {
      const updated = await api.updateTodo(todo.id, { done: !todo.done });
      mutate((prev) => prev?.map((t) => (t.id === updated.id ? updated : t)));
    });

  const remove = (todo: Todo) =>
    run(async () => {
      await api.deleteTodo(todo.id);
      mutate((prev) => prev?.filter((t) => t.id !== todo.id));
    });

  const remaining = () => todos()?.filter((t) => !t.done).length ?? 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Todos</CardTitle>
        <CardDescription>Stored in Postgres, served by Axum, rendered by Solid.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <form onSubmit={add} class="flex gap-2">
          <Input
            value={title()}
            onInput={(e) => setTitle(e.currentTarget.value)}
            placeholder="What needs doing?"
            aria-label="New todo"
            maxLength={200}
          />
          <Button type="submit" disabled={!title().trim()}>
            <Plus /> Add
          </Button>
        </form>

        <Show when={error()}>
          {(msg) => (
            <p role="alert" class="text-sm text-destructive">
              {msg()}
            </p>
          )}
        </Show>

        <Show
          when={!todos.error}
          fallback={<p class="text-sm text-destructive">Couldn't reach the API.</p>}
        >
          <Show
            when={todos()?.length}
            fallback={
              <p class="py-6 text-center text-sm text-muted-foreground">
                {todos.loading ? "Loading…" : "Nothing yet. Add your first todo above."}
              </p>
            }
          >
            <ul class="divide-y rounded-md border">
              <For each={todos()}>
                {(todo) => (
                  <li class="flex items-center gap-3 px-3 py-2">
                    <input
                      type="checkbox"
                      checked={todo.done}
                      onChange={() => toggle(todo)}
                      aria-label={`Mark "${todo.title}" done`}
                      class="size-4 accent-primary"
                    />
                    {/* Text is escaped by default; avoid innerHTML with user data. */}
                    <span
                      class="flex-1 text-sm"
                      classList={{ "text-muted-foreground line-through": todo.done }}
                    >
                      {todo.title}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => remove(todo)}
                      aria-label={`Delete "${todo.title}"`}
                    >
                      <Trash />
                    </Button>
                  </li>
                )}
              </For>
            </ul>
            <p class="text-xs text-muted-foreground">{remaining()} left</p>
          </Show>
        </Show>
      </CardContent>
    </Card>
  );
}
