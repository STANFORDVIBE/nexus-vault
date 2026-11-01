import { useCallback } from "react";
import { Outlet, useLocation, useNavigate } from "react-router";
import { Sidebar } from "@/components/layout/Sidebar";
import { useActivityReporter, useQuickSearchEvent } from "@/hooks/useActivityReporter";
import { useAppHotkeys } from "@/hooks/useAppHotkeys";

const LIST_PREFIXES = ["/items", "/passwords", "/notes", "/favorites", "/trash", "/folders/"];

export function AppShell() {
  const navigate = useNavigate();
  const { pathname } = useLocation();

  useAppHotkeys();
  useActivityReporter();
  useQuickSearchEvent(
    useCallback(() => {
      if (!LIST_PREFIXES.some((p) => pathname.startsWith(p))) void navigate("/items");
    }, [navigate, pathname]),
  );

  return (
    <div className="flex h-full overflow-hidden">
      <Sidebar />
      <main className="min-w-0 flex-1 overflow-y-auto bg-background">
        <Outlet />
      </main>
    </div>
  );
}
