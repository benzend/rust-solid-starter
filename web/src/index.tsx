import { Route, Router } from "@solidjs/router";
import { lazy } from "solid-js";
import { render } from "solid-js/web";
import Layout from "@/components/Layout";
import RequireAuth from "@/components/RequireAuth";
import { AuthProvider } from "@/lib/auth";
import { reportError } from "@/lib/errors";
import Home from "@/routes/Home";
import "./index.css";

// Lazy routes are split into their own chunks and loaded on first visit.
const About = lazy(() => import("@/routes/About"));
const NotFound = lazy(() => import("@/routes/NotFound"));
const Login = lazy(() => import("@/routes/Login"));
const Signup = lazy(() => import("@/routes/Signup"));
const ForgotPassword = lazy(() => import("@/routes/ForgotPassword"));
const ResetPassword = lazy(() => import("@/routes/ResetPassword"));

// Anything thrown outside a component or a handled promise.
window.addEventListener("error", (event) => reportError(event.error ?? event.message));
window.addEventListener("unhandledrejection", (event) => reportError(event.reason));

const root = document.getElementById("root");
if (!root) throw new Error("#root element not found");

render(
  () => (
    <AuthProvider>
      <Router root={Layout}>
        {/* Routes inside RequireAuth need a login. */}
        <Route component={RequireAuth}>
          <Route path="/" component={Home} />
        </Route>
        <Route path="/about" component={About} />
        <Route path="/login" component={Login} />
        <Route path="/signup" component={Signup} />
        <Route path="/forgot-password" component={ForgotPassword} />
        <Route path="/reset-password" component={ResetPassword} />
        <Route path="*404" component={NotFound} />
      </Router>
    </AuthProvider>
  ),
  root,
);
