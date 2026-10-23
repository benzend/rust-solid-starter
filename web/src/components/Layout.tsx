import { A, type RouteSectionProps } from "@solidjs/router";

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
      <main class="mx-auto max-w-2xl px-4 py-10">{props.children}</main>
    </div>
  );
}
