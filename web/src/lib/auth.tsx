// Who's logged in, shared with every component through context.
import { createContext, createResource, type ParentProps, useContext } from "solid-js";
import { api } from "@/lib/api";
import { ApiRequestError } from "@/lib/errors";
import type { User } from "@/types/User";

type Auth = {
  /** The user, `null` when logged out, `undefined` while loading or if the check failed. */
  user: () => User | null | undefined;
  /** True when the API couldn't be reached to check. */
  failed: () => boolean;
  /** Call after logging in, signing up or resetting a password. */
  setUser: (user: User) => void;
  logout: () => Promise<void>;
};

const AuthContext = createContext<Auth>();

export function AuthProvider(props: ParentProps) {
  const [current, { mutate }] = createResource(async () => {
    try {
      return await api.me();
    } catch (err) {
      if (err instanceof ApiRequestError && err.status === 401) return null;
      throw err;
    }
  });

  const auth: Auth = {
    user: () => (current.error ? undefined : current()),
    failed: () => Boolean(current.error),
    setUser: (user) => mutate(user),
    logout: async () => {
      await api.logout();
      mutate(null);
    },
  };
  return <AuthContext.Provider value={auth}>{props.children}</AuthContext.Provider>;
}

export function useAuth(): Auth {
  const auth = useContext(AuthContext);
  if (!auth) throw new Error("useAuth() must be used inside <AuthProvider>");
  return auth;
}

/** Where to go after logging in: the `next` path if it's on this site, else home. */
export function safeNext(next: string | string[] | undefined): string {
  const path = Array.isArray(next) ? next[0] : next;
  // "//evil.example" is a link to another site, so only allow a single leading slash.
  return path?.startsWith("/") && !path.startsWith("//") ? path : "/";
}
