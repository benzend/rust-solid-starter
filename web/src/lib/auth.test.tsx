import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { cleanup, fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import Layout from "@/components/Layout";
import RequireAuth from "@/components/RequireAuth";
import { api } from "@/lib/api";
import { AuthProvider, safeNext } from "@/lib/auth";
import { ApiRequestError } from "@/lib/errors";
import Login from "@/routes/Login";
import ResetPassword from "@/routes/ResetPassword";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const ada = { id: 1, email: "ada@example.com" };
const loggedOut = () => new ApiRequestError("Please log in.", 401, "req-1");

/** The app's routes at `path`, with a stand-in for the protected home page. */
function renderAt(path: string) {
  const history = createMemoryHistory();
  history.set({ value: path });
  render(() => (
    <AuthProvider>
      <MemoryRouter root={Layout} history={history}>
        <Route component={RequireAuth}>
          <Route path="/" component={() => <p>Secret todos</p>} />
        </Route>
        <Route path="/login" component={Login} />
        <Route path="/reset-password" component={ResetPassword} />
      </MemoryRouter>
    </AuthProvider>
  ));
  return history;
}

describe("auth", () => {
  it("sends logged-out visitors to log in, then back where they were going", async () => {
    vi.spyOn(api, "me").mockRejectedValue(loggedOut());
    const login = vi.spyOn(api, "login").mockResolvedValue(ada);
    const history = renderAt("/?tab=open");

    expect(await screen.findByRole("button", { name: "Log in" })).toBeTruthy();
    expect(history.get()).toBe("/login?next=%2F%3Ftab%3Dopen");
    expect(screen.queryByText("Secret todos")).toBeNull();

    fireEvent.input(screen.getByLabelText("Email"), { target: { value: "ada@example.com" } });
    fireEvent.input(screen.getByLabelText("Password"), { target: { value: "hunter2hunter2" } });
    fireEvent.click(screen.getByRole("button", { name: "Log in" }));

    expect(await screen.findByText("Secret todos")).toBeTruthy();
    expect(login).toHaveBeenCalledWith({ email: "ada@example.com", password: "hunter2hunter2" });
    expect(history.get()).toBe("/?tab=open");
    expect(screen.getByText("ada@example.com")).toBeTruthy();
  });

  it("shows login errors", async () => {
    vi.spyOn(api, "me").mockRejectedValue(loggedOut());
    vi.spyOn(api, "login").mockRejectedValue(
      new ApiRequestError("Email or password is incorrect.", 401, "req-2"),
    );
    renderAt("/login");

    fireEvent.input(await screen.findByLabelText("Email"), { target: { value: "a@b.co" } });
    fireEvent.input(screen.getByLabelText("Password"), { target: { value: "wrong-password" } });
    fireEvent.click(screen.getByRole("button", { name: "Log in" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Email or password is incorrect.");
  });

  it("logs out", async () => {
    vi.spyOn(api, "me").mockResolvedValue(ada);
    const logout = vi.spyOn(api, "logout").mockResolvedValue(undefined);
    const history = renderAt("/");

    fireEvent.click(await screen.findByRole("button", { name: "Log out" }));

    expect(await screen.findByRole("link", { name: "Log in" })).toBeTruthy();
    expect(logout).toHaveBeenCalled();
    await waitFor(() => expect(history.get()).toBe("/login"));
  });

  it("resets a password with the token from the link", async () => {
    vi.spyOn(api, "me").mockRejectedValue(loggedOut());
    const reset = vi.spyOn(api, "resetPassword").mockResolvedValue(ada);
    renderAt("/reset-password#token=abc123");

    fireEvent.input(await screen.findByLabelText("New password"), {
      target: { value: "a new password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save and log in" }));

    expect(await screen.findByText("Secret todos")).toBeTruthy();
    expect(reset).toHaveBeenCalledWith({ token: "abc123", password: "a new password" });
  });

  it("only redirects to paths on this site after login", () => {
    expect(safeNext("/todos?x=1")).toBe("/todos?x=1");
    expect(safeNext(undefined)).toBe("/");
    expect(safeNext("https://evil.example")).toBe("/");
    expect(safeNext("//evil.example")).toBe("/");
  });
});
