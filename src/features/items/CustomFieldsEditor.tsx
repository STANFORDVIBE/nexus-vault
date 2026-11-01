import { LockIcon, LockOpenIcon, PlusIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PasswordInput } from "@/components/ui/password-input";
import type { CustomField } from "@/types/vault";

const MAX_FIELDS = 50;

interface CustomFieldsEditorProps {
  value: CustomField[];
  onChange: (value: CustomField[]) => void;
  idPrefix: string;
}

/** "Свои поля": label + value rows; a hidden row is stored like a password. */
export function CustomFieldsEditor({ value, onChange, idPrefix }: CustomFieldsEditorProps) {
  function update(index: number, patch: Partial<CustomField>) {
    onChange(value.map((f, i) => (i === index ? { ...f, ...patch } : f)));
  }

  return (
    <div className="grid gap-2">
      <span className="text-sm font-medium">Свои поля</span>
      {value.length === 0 && (
        <p className="text-xs text-muted-foreground">
          Например, секретный вопрос, кодовое слово или номер договора
        </p>
      )}
      {value.map((field, index) => (
        <div key={index} className="flex items-start gap-2">
          <Input
            aria-label={`Название поля ${index + 1}`}
            placeholder="Название"
            value={field.label}
            maxLength={100}
            onChange={(e) => update(index, { label: e.target.value })}
            className="w-40 shrink-0"
          />
          <div className="min-w-0 flex-1">
            {field.hidden ? (
              <PasswordInput
                id={`${idPrefix}-field-${index}`}
                aria-label={`Значение поля ${index + 1}`}
                placeholder="Значение"
                value={field.value}
                onChange={(e) => update(index, { value: e.target.value })}
              />
            ) : (
              <Input
                id={`${idPrefix}-field-${index}`}
                aria-label={`Значение поля ${index + 1}`}
                placeholder="Значение"
                autoComplete="off"
                value={field.value}
                onChange={(e) => update(index, { value: e.target.value })}
              />
            )}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={field.hidden ? "Сделать поле открытым" : "Сделать поле скрытым"}
            aria-pressed={field.hidden}
            title={field.hidden ? "Скрытое поле — как пароль" : "Открытое поле"}
            onClick={() => update(index, { hidden: !field.hidden })}
          >
            {field.hidden ? <LockIcon /> : <LockOpenIcon />}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`Удалить поле ${index + 1}`}
            onClick={() => onChange(value.filter((_, i) => i !== index))}
          >
            <XIcon />
          </Button>
        </div>
      ))}
      {value.length < MAX_FIELDS && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() => onChange([...value, { label: "", value: "", hidden: false }])}
        >
          <PlusIcon />
          Добавить поле
        </Button>
      )}
    </div>
  );
}
