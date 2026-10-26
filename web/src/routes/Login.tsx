import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { createSignal } from "solid-js";
import { AuthCard, Field, FormError } from "@/components/AuthCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { safeNext, useAuth } from "@/lib/auth";
import { createSubmit } from "@/lib/form";

export default function Login() {
  const { setUser } = useAuth();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");

  const { submit, error, pending } = createSubmit(async () => {
    setUser(await api.login({ email: email(), password: password() }));
    navigate(safeNext(params.next), { replace: true });
  });

  return (
    <AuthCard
      title="Log in"
      footer={
        <>
          No account yet?{" "}
          <A href="/signup" class="underline">
            Sign up
          </A>
        </>
      }
    >
      <form onSubmit={submit} class="space-y-4">
        <Field label="Email">
          <Input
            type="email"
            autocomplete="email"
            required
            value={email()}
            onInput={(e) => setEmail(e.currentTarget.value)}
          />
        </Field>
        <Field label="Password">
          <Input
            type="password"
            autocomplete="current-password"
            required
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />
        </Field>
        <FormError message={error()} />
        <Button type="submit" class="w-full" disabled={pending()}>
          Log in
        </Button>
        <A href="/forgot-password" class="block text-center text-sm underline">
          Forgot your password?
        </A>
      </form>
    </AuthCard>
  );
}
