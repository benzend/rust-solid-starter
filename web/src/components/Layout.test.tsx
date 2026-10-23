import { MemoryRouter, Route } from "@solidjs/router";
import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import Layout from "@/components/Layout";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("shows a fallback and reports the error when a page crashes", async () => {
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  const Broken = () => {
    throw new Error("render failed");
  };
  render(() => (
    <MemoryRouter root={Layout}>
      <Route path="/" component={Broken} />
    </MemoryRouter>
  ));

  expect(await screen.findByText("Something went wrong")).toBeTruthy();
  expect(screen.getByRole("link", { name: "Todos" })).toBeTruthy(); // the layout survives
  expect(log).toHaveBeenCalledWith(expect.objectContaining({ message: "render failed" }), {
    source: "ErrorBoundary",
    requestId: undefined,
  });
});
