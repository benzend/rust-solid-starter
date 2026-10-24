// The one place that knows about an error-tracking service. Everything else
// calls reportError(); plugging in Sentry, Honeybadger, etc. means editing
// only this file (see "Adding error tracking" in the README).

/** A non-2xx response from our API, with the server's request ID for lookup in the logs. */
export class ApiRequestError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly requestId: string | null,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }
}

/** Report an unexpected error. 4xx responses are the user's input, not bugs, so they're skipped. */
export function reportError(error: unknown, context?: Record<string, unknown>) {
  if (error instanceof ApiRequestError && error.status < 500) return;
  const requestId = error instanceof ApiRequestError ? error.requestId : undefined;
  console.error(error, { ...context, requestId });
}

/** What to show the user. Server errors include the request ID so a report can be traced. */
export function userMessage(error: unknown): string {
  if (error instanceof ApiRequestError) {
    return error.status >= 500 && error.requestId
      ? `${error.message} (ref ${error.requestId})`
      : error.message;
  }
  return "Something went wrong. Check your connection and try again.";
}
