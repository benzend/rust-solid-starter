import { A } from "@solidjs/router";
import { buttonVariants } from "@/components/ui/button";

export default function NotFound() {
  return (
    <div class="space-y-4 py-16 text-center">
      <h1 class="text-2xl font-semibold">Page not found</h1>
      <p class="text-sm text-muted-foreground">That page doesn't exist.</p>
      <A href="/" class={buttonVariants({ variant: "outline" })}>
        Back home
      </A>
    </div>
  );
}
