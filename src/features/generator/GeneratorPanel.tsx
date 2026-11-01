import { useCallback, useEffect, useState, type ReactNode } from "react";
import { CopyIcon, RefreshCwIcon } from "lucide-react";
import { cn } from "cn";
import { StrengthMeter } from "@/components/common/StrengthMeter";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { errorMessage } from "@/services/errors";
import { generatePassword } from "@/services/vault";
import { entropyLevel, loadOptions, saveOptions } from "@/features/generator/options";
import type { GeneratorMode, GeneratorOptions } from "@/types/vault";

const MODES = [
  { value: "password", label: "Пароль" },
  { value: "passphrase", label: "Фраза" },
] as const;

const CHARSETS = [
  { key: "uppercase", label: "Заглавные A–Z" },
  { key: "lowercase", label: "Строчные a–z" },
  { key: "digits", label: "Цифры 0–9" },
  { key: "symbols", label: "Символы !@#$" },
  { key: "excludeAmbiguous", label: "Без похожих (l, 1, O, 0)" },
] as const;

function ToggleRow({
  id,
  label,
  checked,
  onChange,
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <Label htmlFor={id} className="font-normal">
        {label}
      </Label>
      <Switch id={id} checked={checked} onCheckedChange={onChange} />
    </div>
  );
}

interface GeneratorPanelProps {
  /** Rendered next to "Обновить"; receives the current value (e.g. "Копировать" or "Использовать"). */
  renderActions?: (value: string) => ReactNode;
  onCopy?: (value: string) => void;
  compact?: boolean;
}

export function GeneratorPanel({ renderActions, onCopy, compact }: GeneratorPanelProps) {
  const [options, setOptions] = useState<GeneratorOptions>(loadOptions);
  const [value, setValue] = useState("");
  const [bits, setBits] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const generate = useCallback(async (next: GeneratorOptions) => {
    try {
      const result = await generatePassword(next);
      setValue(result.value);
      setBits(result.entropyBits);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    saveOptions(options);
    generatePassword(options)
      .then((result) => {
        if (cancelled) return;
        setValue(result.value);
        setBits(result.entropyBits);
        setError(null);
      })
      .catch((e: unknown) => !cancelled && setError(errorMessage(e)));
    return () => {
      cancelled = true;
    };
  }, [options]);

  const patch = (change: Partial<GeneratorOptions>) => setOptions((o) => ({ ...o, ...change }));
  const strength = entropyLevel(bits);
  const idp = compact ? "gen-c" : "gen";

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-3">
        <div
          className={cn(
            "rounded-md border border-border bg-muted/40 px-3 py-3 font-mono break-all select-all",
            compact ? "min-h-12 text-sm" : "min-h-16 text-base",
          )}
          aria-live="polite"
          data-testid="generated-value"
        >
          {value || " "}
        </div>
        {error ? (
          <p className="text-xs text-destructive">{error}</p>
        ) : (
          <StrengthMeter level={strength.level} label={strength.label} hint={`≈ ${bits} бит`} />
        )}
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={() => void generate(options)}>
            <RefreshCwIcon />
            Обновить
          </Button>
          {onCopy && (
            <Button variant="outline" onClick={() => onCopy(value)} disabled={!value}>
              <CopyIcon />
              Копировать
            </Button>
          )}
          {renderActions?.(value)}
        </div>
      </div>

      <SegmentedControl
        aria-label="Тип"
        value={options.mode}
        options={MODES}
        onValueChange={(mode: GeneratorMode) => patch({ mode })}
        className="self-start"
      />

      {options.mode === "password" ? (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <Label htmlFor={`${idp}-length`}>Длина</Label>
              <span className="text-sm font-medium tabular-nums">{options.length}</span>
            </div>
            <Slider
              id={`${idp}-length`}
              min={8}
              max={128}
              step={1}
              value={[options.length]}
              onValueChange={([length]) => length !== undefined && patch({ length })}
              aria-label="Длина пароля"
            />
          </div>
          {CHARSETS.map(({ key, label }) => (
            <ToggleRow
              key={key}
              id={`${idp}-${key}`}
              label={label}
              checked={options[key]}
              onChange={(checked) => patch({ [key]: checked })}
            />
          ))}
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <Label htmlFor={`${idp}-words`}>Количество слов</Label>
              <span className="text-sm font-medium tabular-nums">{options.words}</span>
            </div>
            <Slider
              id={`${idp}-words`}
              min={3}
              max={12}
              step={1}
              value={[options.words]}
              onValueChange={([words]) => words !== undefined && patch({ words })}
              aria-label="Количество слов"
            />
          </div>
          <div className="flex items-center justify-between gap-4">
            <Label htmlFor={`${idp}-separator`} className="font-normal">
              Разделитель
            </Label>
            <Input
              id={`${idp}-separator`}
              value={options.separator}
              maxLength={3}
              onChange={(e) => patch({ separator: e.target.value })}
              className="h-8 w-16 text-center font-mono"
            />
          </div>
          <ToggleRow
            id={`${idp}-capitalize`}
            label="С заглавной буквы"
            checked={options.capitalize}
            onChange={(capitalize) => patch({ capitalize })}
          />
          <ToggleRow
            id={`${idp}-number`}
            label="Добавить число"
            checked={options.includeNumber}
            onChange={(includeNumber) => patch({ includeNumber })}
          />
        </div>
      )}
    </div>
  );
}
