import { type JSX, splitProps } from "solid-js";
import { cn } from "@/lib/utils";

type DivProps = JSX.HTMLAttributes<HTMLDivElement>;

function withClass(base: string) {
  return (props: DivProps) => {
    const [local, rest] = splitProps(props, ["class"]);
    return <div class={cn(base, local.class)} {...rest} />;
  };
}

export const Card = withClass(
  "flex flex-col gap-6 rounded-xl border bg-card py-6 text-card-foreground shadow-sm",
);
export const CardHeader = withClass("grid auto-rows-min items-start gap-1.5 px-6");
export const CardTitle = withClass("font-semibold leading-none");
export const CardDescription = withClass("text-sm text-muted-foreground");
export const CardContent = withClass("px-6");
export const CardFooter = withClass("flex items-center px-6");
