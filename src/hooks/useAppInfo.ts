import { useEffect, useState } from "react";
import { getAppInfo } from "@/services/app";
import type { AppInfo } from "@/types/app";

/** App name and version from the Rust core; `null` while loading or if IPC is unavailable. */
export function useAppInfo(): AppInfo | null {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((result) => {
        if (!cancelled) setInfo(result);
      })
      .catch((error: unknown) => console.error("get_app_info failed", error));
    return () => {
      cancelled = true;
    };
  }, []);

  return info;
}
