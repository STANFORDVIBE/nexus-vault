import { useState } from "react";
import { FileUpIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { importMessage } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { importItems } from "@/services/vault";
import { invalidate } from "@/store/session";

/** Adds entries from a CSV/JSON export of another manager; nothing is overwritten. */
export function ImportItems() {
  const [busy, setBusy] = useState(false);

  async function run() {
    setBusy(true);
    try {
      const summary = await importItems();
      if (summary) {
        invalidate();
        const { title, description } = importMessage(summary);
        toast.success(title, { description, duration: 10_000 });
      }
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Button variant="outline" disabled={busy} onClick={() => void run()}>
      <FileUpIcon />
      {busy ? "Импортирую…" : "Импорт…"}
    </Button>
  );
}
