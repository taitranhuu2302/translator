import { useEffect, useState } from "react";

import { ipc, type ProviderStatus } from "./ipc";

/**
 * Groundwork shell. Ticket 01 ships no feature UI; this page proves the one
 * frontend seam — every command goes through `ipc` — in the packaged bundle.
 */
export function App() {
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipc
      .providerStatus()
      .then((result) => {
        if (!cancelled) setStatus(result);
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(JSON.stringify(reason));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="shell">
      <h1>Translator</h1>
      <p className="muted">Groundwork build — no features yet.</p>
      {error ? (
        <p className="error">Could not read provider status: {error}</p>
      ) : status ? (
        <p className="muted">
          Azure key: {status.azure_configured ? "configured" : "missing"} · Groq
          key: {status.groq_configured ? "configured" : "missing"}
        </p>
      ) : (
        <p className="muted">Checking providers…</p>
      )}
    </main>
  );
}
