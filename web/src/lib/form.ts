import { createSignal } from "solid-js";
import { reportError, userMessage } from "@/lib/errors";

/** Submit handling for forms: prevents the page reload, tracks pending, and shows errors. */
export function createSubmit(action: () => Promise<void>) {
  const [error, setError] = createSignal<string | null>(null);
  const [pending, setPending] = createSignal(false);

  const submit = async (e: SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setPending(true);
    try {
      await action();
    } catch (err) {
      setError(userMessage(err));
      reportError(err);
    } finally {
      setPending(false);
    }
  };
  return { submit, error, pending };
}
