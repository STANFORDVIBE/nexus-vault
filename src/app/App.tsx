import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { NAV_SECTIONS, SETTINGS_NAV } from "@/app/navigation";
import { SectionPlaceholder } from "@/app/SectionPlaceholder";
import { AppShell } from "@/components/layout/AppShell";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { DesignSystemPage } from "@/features/design-system/DesignSystemPage";

const PLACEHOLDER_ROUTES = [...NAV_SECTIONS.flatMap((section) => section.items), SETTINGS_NAV];

export function App() {
  return (
    <TooltipProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<Navigate to="/design-system" replace />} />
            <Route path="design-system" element={<DesignSystemPage />} />
            {PLACEHOLDER_ROUTES.map((item) => (
              <Route key={item.to} path={item.to} element={<SectionPlaceholder />} />
            ))}
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
      <Toaster />
    </TooltipProvider>
  );
}
