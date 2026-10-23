import { Route, Router } from "@solidjs/router";
import { lazy } from "solid-js";
import { render } from "solid-js/web";
import Layout from "@/components/Layout";
import { reportError } from "@/lib/errors";
import Home from "@/routes/Home";
import "./index.css";

// Lazy routes are split into their own chunks and loaded on first visit.
const About = lazy(() => import("@/routes/About"));
const NotFound = lazy(() => import("@/routes/NotFound"));

// Anything thrown outside a component or a handled promise.
window.addEventListener("error", (event) => reportError(event.error ?? event.message));
window.addEventListener("unhandledrejection", (event) => reportError(event.reason));

const root = document.getElementById("root");
if (!root) throw new Error("#root element not found");

render(
  () => (
    <Router root={Layout}>
      <Route path="/" component={Home} />
      <Route path="/about" component={About} />
      <Route path="*404" component={NotFound} />
    </Router>
  ),
  root,
);
