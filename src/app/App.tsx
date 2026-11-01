import { BrowserRouter, Route, Routes } from "react-router";
import { StartupScreen } from "@/app/StartupScreen";

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="*" element={<StartupScreen />} />
      </Routes>
    </BrowserRouter>
  );
}
