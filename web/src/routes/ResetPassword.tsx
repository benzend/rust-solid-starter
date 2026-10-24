import { A, useLocation, useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { AuthCard, Field, FormError } from "@/components/AuthCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { createSubmit } from "@/lib/form";

export default function ResetPassword() {
  const { setUser } = useAuth();
  const navigate = useNavigate();
  // The emailed link carries the token after `#`, which browsers don't send to
  // the server, so it stays out of logs.
  const token = new URLSearchParams(useLocation().hash.slice(1)).get("token");
  const [password, setPassword] = createSignal("");

  const { submit, error, pending } = createSubmit(async () => {
    setUser(await api.resetPassword({ token: token ?? "", password: password() }));
    navigate("/", { replace: true });
  });

  return (
    <AuthCard title="Choose a new password">
      <Show
        when={token}
        fallback={
          <p class="text-sm">
            This link is missing its token.{" "}
            <A href="/forgot-password" class="underline">
              Request a new one
            </A>
            .
          </p>
        }
      >
        <form onSubmit={submit} class="space-y-4">
          <Field label="New password">
            <Input
              type="password"
              autocomplete="new-password"
              required
              minLength={8}
              maxLength={128}
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
            />
          </Field>
          <FormError message={error()} />
          <Button type="submit" class="w-full" disabled={pending()}>
            Save and log in
          </Button>
        </form>
      </Show>
    </AuthCard>
  );
}
