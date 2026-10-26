import { A, useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { AuthCard, Field, FormError } from "@/components/AuthCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { createSubmit } from "@/lib/form";

export default function Signup() {
  const { setUser } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");

  const { submit, error, pending } = createSubmit(async () => {
    setUser(await api.signup({ email: email(), password: password() }));
    navigate("/", { replace: true });
  });

  return (
    <AuthCard
      title="Create an account"
      footer={
        <>
          Already have one?{" "}
          <A href="/login" class="underline">
            Log in
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
            autocomplete="new-password"
            required
            minLength={8}
            maxLength={128}
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />
        </Field>
        <p class="text-xs text-muted-foreground">At least 8 characters.</p>
        <FormError message={error()} />
        <Button type="submit" class="w-full" disabled={pending()}>
          Sign up
        </Button>
      </form>
    </AuthCard>
  );
}
