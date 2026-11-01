import { useEffect, useState } from "react";
import { getAppInfo } from "@/services/app";
import type { AppInfo } from "@/types/app";

type BackendState =
  { status: "loading" } | { status: "ready"; info: AppInfo } | { status: "error" };

/**
 * Foundation screen: proves the WebView boots and the IPC bridge to Rust works.
 * Replaced by Onboarding / Unlock once the Security Core is in place.
 */
export function StartupScreen() {
  const [backend, setBackend] = useState<BackendState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((info) => !cancelled && setBackend({ status: "ready", info }))
      .catch(() => !cancelled && setBackend({ status: "error" }));
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-3">
      <h1 className="text-2xl font-semibold tracking-tight">Nexus Vault</h1>
      <p className="text-sm text-text-secondary">Your digital life. One secure place.</p>
      <p className="mt-4 text-xs text-text-secondary" role="status">
        {backend.status === "loading" && "Подключение к ядру…"}
        {backend.status === "ready" && (
          <>
            <span className="text-success">●</span> Ядро на связи · v{backend.info.version}
          </>
        )}
        {backend.status === "error" && (
          <>
            <span className="text-danger">●</span> Ядро недоступно
          </>
        )}
      </p>
    </main>
  );
}
