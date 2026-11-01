import { render, screen } from "@testing-library/react";
import { StartupScreen } from "@/app/StartupScreen";
import { getAppInfo } from "@/services/app";

vi.mock("@/services/app", () => ({ getAppInfo: vi.fn() }));

const mockedGetAppInfo = vi.mocked(getAppInfo);

describe("StartupScreen", () => {
  it("shows the backend version once IPC answers", async () => {
    mockedGetAppInfo.mockResolvedValue({ name: "Nexus Vault", version: "0.1.0" });

    render(<StartupScreen />);

    expect(screen.getByRole("heading", { name: "Nexus Vault" })).toBeInTheDocument();
    expect(await screen.findByText(/v0\.1\.0/)).toBeInTheDocument();
  });

  it("reports an unreachable backend", async () => {
    mockedGetAppInfo.mockRejectedValue(new Error("ipc down"));

    render(<StartupScreen />);

    expect(await screen.findByText("Ядро недоступно")).toBeInTheDocument();
  });
});
