import { useCallback, useEffect, useRef, useState } from "react";
import { useSessionStore } from "@/store/session";

interface QueryResult<T> {
  data: T | undefined;
  error: unknown;
  loading: boolean;
  reload: () => void;
}

/**
 * Minimal data hook for IPC reads. Refetches when `key` changes or when any mutation
 * bumps the session revision. Stale responses (an older key) are discarded.
 */
export function useVaultQuery<T>(key: string, fetcher: () => Promise<T>): QueryResult<T> {
  const revision = useSessionStore((s) => s.revision);
  const fetcherRef = useRef(fetcher);
  // runs before the fetch effect below, so it always calls the latest fetcher
  useEffect(() => {
    fetcherRef.current = fetcher;
  });

  const [state, setState] = useState<{ key: string; data?: T; error: unknown }>({
    key: "",
    error: null,
  });
  const [nonce, setNonce] = useState(0);
  const reload = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;
    fetcherRef
      .current()
      .then((data) => {
        if (!cancelled) setState({ key, data, error: null });
      })
      .catch((error: unknown) => {
        if (!cancelled) setState((prev) => ({ key, data: prev.data, error }));
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision, nonce]);

  return {
    // keep showing the previous key's data while the new one loads (no flicker)
    data: state.data,
    error: state.error,
    loading: state.key !== key,
    reload,
  };
}
