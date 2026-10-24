import { A, type RouteSectionProps, useNavigate } from "@solidjs/router";
import { ErrorBoundary, Show } from "solid-js";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/lib/auth";
import { reportError } from "@/lib/errors";

export default function Layout(props: RouteSectionProps) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
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
          <div class="ml-auto flex items-center gap-3">
            <Show
              when={user()}
              fallback={
                <Show when={user() === null}>
                  <A href="/login" {...nav}>
                    Log in
                  </A>
                </Show>
              }
            >
              {(u) => (
                <>
                  <span class="text-muted-foreground">{u().email}</span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={async () => {
                      await logout();
                      navigate("/login");
                    }}
                  >
                    Log out
                  </Button>
                </>
              )}
            </Show>
          </div>
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
