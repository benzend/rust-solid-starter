import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/api";

const respond = (status: number, body?: unknown) =>
  vi.fn().mockResolvedValue(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("api", () => {
  it("sends JSON and returns the parsed body", async () => {
    const fetch = respond(201, { id: 1, title: "Write tests", done: false });
    vi.stubGlobal("fetch", fetch);

    const todo = await api.createTodo({ title: "Write tests" });

    expect(todo).toEqual({ id: 1, title: "Write tests", done: false });
    expect(fetch).toHaveBeenCalledWith(
      "/api/todos",
      expect.objectContaining({ method: "POST", body: '{"title":"Write tests"}' }),
    );
  });

  it("throws the server's ApiError message", async () => {
    vi.stubGlobal("fetch", respond(422, { message: "Title must be 1-200 characters." }));

    await expect(api.createTodo({ title: " " })).rejects.toThrow("Title must be 1-200 characters.");
  });

  it("returns undefined for 204 No Content", async () => {
    vi.stubGlobal("fetch", respond(204));

    await expect(api.deleteTodo(1)).resolves.toBeUndefined();
  });
});
