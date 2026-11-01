import { toast } from "sonner";
import { errorMessage } from "@/services/errors";
import { installUpdate } from "@/services/vault";

/** Starts the update: the app locks, closes, installs and opens again. */
export async function runUpdate() {
  const id = toast.loading("Скачиваем обновление…", {
    description: "Приложение закроется и откроется уже новой версией",
  });
  try {
    await installUpdate();
  } catch (e) {
    toast.error(errorMessage(e), { id, description: undefined });
  }
}
