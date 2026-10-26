import { type RouteSectionProps, useNavigate } from "@solidjs/router";
import { createEffect, Match, Switch } from "solid-js";
import { useAuth } from "@/lib/auth";

/** Wrap routes in this to require a login. Logged-out visitors go to /login and come back after. */
export default function RequireAuth(props: RouteSectionProps) {
  const { user, failed } = useAuth();
  const navigate = useNavigate();

  createEffect(() => {
    if (user() === null) {
      const next = props.location.pathname + props.location.search;
      navigate(`/login?next=${encodeURIComponent(next)}`, { replace: true });
    }
  });

  return (
    <Switch>
      <Match when={failed()}>
        <p class="py-16 text-center text-sm text-destructive">Couldn't reach the API.</p>
      </Match>
      <Match when={user()}>{props.children}</Match>
    </Switch>
  );
}
