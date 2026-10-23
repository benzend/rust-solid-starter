import { A, type RouteSectionProps } from "@solidjs/router";
import { ErrorBoundary } from "solid-js";
import { Button } from "@/components/ui/button";
import { reportError } from "@/lib/errors";

export default function Layout(props: RouteSectionProps) {
  const nav = {
    class: "transition-colors hover:text-foreground",
    activeClass: "text-foreground",
    inactiveClass: "text-muted-foreground",
  };
  return (
    <div class="min-h-dvh">
      <header class="border-b">
        <nav class="mx-auto flex h-14 max-w-2xl items-center gap-6 px-4 text-sm">
          <A href="/" class="font-semibold" activeClass="" inactiveClass="">
            Rust + Solid
          </A>
          <A href="/" end {...nav}>
            Todos
          </A>
          <A href="/about" {...nav}>
            About
          </A>
        </nav>
      </header>
      <main class="mx-auto max-w-2xl px-4 py-10">
        {/* A crash while rendering a page shows this instead of a blank screen. */}
        <ErrorBoundary fallback={(error, reset) => <Crashed error={error} reset={reset} />}>
          {props.children}
        </ErrorBoundary>
      </main>
    </div>
  );
}

function Crashed(props: { error: unknown; reset: () => void }) {
  reportError(props.error, { source: "ErrorBoundary" });
  return (
    <div role="alert" class="space-y-4 py-16 text-center">
      <h1 class="text-2xl font-semibold">Something went wrong</h1>
      <p class="text-sm text-muted-foreground">This page hit an unexpected error.</p>
      <Button variant="outline" onClick={() => props.reset()}>
        Try again
      </Button>
    </div>
  );
}
