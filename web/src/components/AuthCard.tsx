import type { JSX, ParentProps } from "solid-js";
import { Show } from "solid-js";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

/** The card the login, sign-up and password pages share. */
export function AuthCard(
  props: ParentProps<{ title: string; description?: string; footer?: JSX.Element }>,
) {
  return (
    <Card class="mx-auto max-w-sm">
      <CardHeader>
        <CardTitle>{props.title}</CardTitle>
        <Show when={props.description}>
          <CardDescription>{props.description}</CardDescription>
        </Show>
      </CardHeader>
      <CardContent class="space-y-4">
        {props.children}
        <Show when={props.footer}>
          <p class="text-sm text-muted-foreground">{props.footer}</p>
        </Show>
      </CardContent>
    </Card>
  );
}

export function Field(props: ParentProps<{ label: string }>) {
  return (
    // biome-ignore lint/a11y/noLabelWithoutControl: the input is passed in as children
    <label class="block space-y-1.5 text-sm font-medium">
      <span>{props.label}</span>
      {props.children}
    </label>
  );
}

export function FormError(props: { message: string | null }) {
  return (
    <Show when={props.message}>
      {(msg) => (
        <p role="alert" class="text-sm text-destructive">
          {msg()}
        </p>
      )}
    </Show>
  );
}
