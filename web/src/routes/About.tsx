import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export default function About() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>About this starter</CardTitle>
      </CardHeader>
      <CardContent class="space-y-3 text-sm text-muted-foreground">
        <p>
          Axum serves a JSON API under <code class="text-foreground">/api</code> and this Solid app
          everywhere else. Types are generated from the Rust structs, so the two sides can't drift.
        </p>
        <p>This page is a lazy-loaded route: it ships as its own small chunk.</p>
      </CardContent>
    </Card>
  );
}
