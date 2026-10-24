import { A } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { AuthCard, Field, FormError } from "@/components/AuthCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { createSubmit } from "@/lib/form";

export default function ForgotPassword() {
  const [email, setEmail] = createSignal("");
  const [sent, setSent] = createSignal(false);

  const { submit, error, pending } = createSubmit(async () => {
    await api.forgotPassword({ email: email() });
    setSent(true);
  });

  return (
    <AuthCard
      title="Reset your password"
      description="We'll email you a link to choose a new one."
      footer={
        <A href="/login" class="underline">
          Back to log in
        </A>
      }
    >
      <Show
        when={!sent()}
        fallback={
          // The same message whether or not the account exists, like the API.
          <p role="status" class="text-sm">
            If there's an account for {email()}, a reset link is on its way. It expires in an hour.
          </p>
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
          <FormError message={error()} />
          <Button type="submit" class="w-full" disabled={pending()}>
            Send reset link
          </Button>
        </form>
      </Show>
    </AuthCard>
  );
}
