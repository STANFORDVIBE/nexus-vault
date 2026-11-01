import { FolderIcon } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { listFolders } from "@/services/vault";

// Radix Select does not allow an empty string as an item value
const NONE = "__none__";

interface FolderSelectProps {
  id?: string;
  value: string | null;
  onChange: (folderId: string | null) => void;
  disabled?: boolean;
}

export function FolderSelect({ id, value, onChange, disabled }: FolderSelectProps) {
  const { data: folders } = useVaultQuery("folders", listFolders);

  return (
    <Select
      value={value ?? NONE}
      onValueChange={(next) => onChange(next === NONE ? null : next)}
      disabled={disabled}
    >
      <SelectTrigger id={id} className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={NONE}>Без папки</SelectItem>
        {!!folders?.length && <SelectSeparator />}
        {folders?.map((folder) => (
          <SelectItem key={folder.id} value={folder.id}>
            <FolderIcon className="text-muted-foreground" />
            {folder.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
