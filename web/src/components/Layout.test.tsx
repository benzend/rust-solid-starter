import { MemoryRouter, Route } from "@solidjs/router";
import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import Layout from "@/components/Layout";
import { api } from "@/lib/api";
import { AuthProvider } from "@/lib/auth";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("shows a fallback and reports the error when a page crashes", async () => {
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  const Broken = () => {
    throw new Error("render failed");
  };
  vi.spyOn(api, "me").mockResolvedValue({ id: 1, email: "ada@example.com" });
  render(() => (
    <AuthProvider>
      <MemoryRouter root={Layout}>
        <Route path="/" component={Broken} />
      </MemoryRouter>
    </AuthProvider>
  ));

  expect(await screen.findByText("Something went wrong")).toBeTruthy();
  expect(screen.getByRole("link", { name: "Todos" })).toBeTruthy(); // the layout survives
  expect(log).toHaveBeenCalledWith(expect.objectContaining({ message: "render failed" }), {
    source: "ErrorBoundary",
    requestId: undefined,
  });
});
