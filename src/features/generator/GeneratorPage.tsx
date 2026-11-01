import { ShieldCheckIcon } from "lucide-react";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardContent } from "@/components/ui/card";
import { GeneratorPanel } from "@/features/generator/GeneratorPanel";
import { useCopy } from "@/hooks/useCopy";

export function GeneratorPage() {
  const { copyValue } = useCopy();

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-6 px-8 py-8">
      <PageHeader
        title="Генератор паролей"
        description="Случайные пароли и парольные фразы создаются криптостойким генератором на этом компьютере"
      />
      <Card>
        <CardContent>
          <GeneratorPanel onCopy={(value) => void copyValue(value)} />
        </CardContent>
      </Card>
      <p className="flex items-center gap-2 text-xs text-muted-foreground">
        <ShieldCheckIcon className="size-3.5" />
        Сгенерированный пароль нигде не сохраняется, пока вы не добавите его в запись
      </p>
    </div>
  );
}
