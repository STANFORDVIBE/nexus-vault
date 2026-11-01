import { toast } from "sonner";
import { errorMessage } from "@/services/errors";
import { createItem, setFavorite, updateItem } from "@/services/vault";
import { invalidate } from "@/store/session";
import type { ItemInput } from "@/types/vault";

/** Create or update an item from an editor form; returns the id, or null on error. */
export async function saveItem(
  input: ItemInput,
  opts: { itemId?: string; initialFavorite?: boolean; createdMessage: string },
): Promise<string | null> {
  try {
    if (opts.itemId) {
      await updateItem(opts.itemId, input);
      if (!!input.favorite !== !!opts.initialFavorite)
        await setFavorite(opts.itemId, !!input.favorite);
      invalidate();
      toast.success("Изменения сохранены");
      return opts.itemId;
    }
    const id = await createItem(input);
    invalidate();
    toast.success(opts.createdMessage);
    return id;
  } catch (e) {
    toast.error(errorMessage(e));
    return null;
  }
}
