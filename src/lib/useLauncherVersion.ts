import { useEffect, useState } from "react";

import { getLauncherVersion } from "./tauri";

/** Fetches the launcher version once via the real `get_launcher_version` IPC command. */
export function useLauncherVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getLauncherVersion()
      .then((value) => {
        if (!cancelled) setVersion(value);
      })
      .catch(() => {
        if (!cancelled) setVersion(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return version;
}
