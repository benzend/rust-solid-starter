import { fireEvent, render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/api";
import Home from "@/routes/Home";

vi.mock("@/lib/api", () => ({
  api: {
    listTodos: vi.fn(),
    createTodo: vi.fn(),
    updateTodo: vi.fn(),
    deleteTodo: vi.fn(),
  },
}));

beforeEach(() => {
  vi.resetAllMocks();
});

describe("Home", () => {
  it("shows the empty state, then adds a todo", async () => {
    vi.mocked(api.listTodos).mockResolvedValue([]);
    vi.mocked(api.createTodo).mockResolvedValue({ id: 1, title: "Ship it", done: false });
    render(() => <Home />);

    expect(await screen.findByText("Nothing yet. Add your first todo above.")).toBeTruthy();

    fireEvent.input(screen.getByLabelText("New todo"), { target: { value: "Ship it" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    expect(await screen.findByText("Ship it")).toBeTruthy();
    expect(api.createTodo).toHaveBeenCalledWith({ title: "Ship it" });
    expect(screen.getByText("1 left")).toBeTruthy();
  });

  it("shows the API's error message when an action fails", async () => {
    vi.mocked(api.listTodos).mockResolvedValue([{ id: 1, title: "Ship it", done: false }]);
    vi.mocked(api.deleteTodo).mockRejectedValue(new Error("Not found."));
    render(() => <Home />);

    fireEvent.click(await screen.findByRole("button", { name: 'Delete "Ship it"' }));

    expect((await screen.findByRole("alert")).textContent).toBe("Not found.");
  });
});
