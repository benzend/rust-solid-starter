import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError, reportError, userMessage } from "@/lib/errors";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("reportError", () => {
  it("reports server errors with their request ID", () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    const error = new ApiRequestError("Something went wrong.", 500, "req-1");

    reportError(error, { source: "test" });

    expect(log).toHaveBeenCalledWith(error, { source: "test", requestId: "req-1" });
  });

  it("skips 4xx responses, which are the user's input", () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});

    reportError(new ApiRequestError("Title must be 1-200 characters.", 422, "req-2"));

    expect(log).not.toHaveBeenCalled();
  });
});

describe("userMessage", () => {
  it("adds the request ID to server errors", () => {
    expect(userMessage(new ApiRequestError("Something went wrong.", 500, "req-3"))).toBe(
      "Something went wrong. (ref req-3)",
    );
  });

  it("shows client errors as-is", () => {
    expect(userMessage(new ApiRequestError("Not found.", 404, "req-4"))).toBe("Not found.");
  });

  it("falls back for network failures", () => {
    expect(userMessage(new TypeError("Failed to fetch"))).toMatch(/connection/);
  });
});
