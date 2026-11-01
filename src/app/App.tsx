import { BrowserRouter, Navigate, Route, Routes, useParams } from "react-router";
import { AppShell } from "@/components/layout/AppShell";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { VaultGate } from "@/features/auth/VaultGate";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { FoldersPage } from "@/features/folders/FoldersPage";
import { GeneratorPage } from "@/features/generator/GeneratorPage";
import { HealthPage } from "@/features/health/HealthPage";
import { ItemsPage } from "@/features/items/ItemsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";

/** Remounts the list per folder so search and selection start fresh. */
function FolderItems() {
  const { folderId } = useParams();
  return <ItemsPage key={folderId} view="folder" />;
}

export function App() {
  return (
    <TooltipProvider>
      <VaultGate>
        <BrowserRouter>
          <Routes>
            <Route element={<AppShell />}>
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="dashboard" element={<DashboardPage />} />
              <Route path="items" element={<ItemsPage key="all" view="all" />} />
              <Route path="passwords" element={<ItemsPage key="logins" view="logins" />} />
              <Route path="notes" element={<ItemsPage key="notes" view="notes" />} />
              <Route path="cards" element={<ItemsPage key="cards" view="cards" />} />
              <Route path="documents" element={<ItemsPage key="identities" view="identities" />} />
              <Route path="favorites" element={<ItemsPage key="favorites" view="favorites" />} />
              <Route path="trash" element={<ItemsPage key="trash" view="trash" />} />
              <Route path="folders" element={<FoldersPage />} />
              <Route path="folders/:folderId" element={<FolderItems />} />
              <Route path="generator" element={<GeneratorPage />} />
              <Route path="health" element={<HealthPage />} />
              <Route path="settings" element={<SettingsPage />} />
              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Route>
          </Routes>
        </BrowserRouter>
      </VaultGate>
      <Toaster />
    </TooltipProvider>
  );
}
