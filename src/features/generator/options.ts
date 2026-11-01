import type { StrengthLevel } from "@/lib/password-strength";
import type { GeneratorOptions } from "@/types/vault";

// Options are a non-secret preference; the generated value itself is never stored.
const STORAGE_KEY = "nexus-vault:generator";

export const DEFAULT_OPTIONS: GeneratorOptions = {
  mode: "password",
  length: 20,
  uppercase: true,
  lowercase: true,
  digits: true,
  symbols: true,
  excludeAmbiguous: false,
  words: 5,
  separator: "-",
  capitalize: true,
  includeNumber: false,
};

export function loadOptions(): GeneratorOptions {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...DEFAULT_OPTIONS, ...(JSON.parse(raw) as Partial<GeneratorOptions>) };
  } catch {
    // ignore broken or unavailable storage
  }
  return DEFAULT_OPTIONS;
}

export function saveOptions(options: GeneratorOptions) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
  } catch {
    // not persisted
  }
}

export function entropyLevel(bits: number): { level: StrengthLevel; label: string } {
  if (bits < 40) return { level: 1, label: "Слабый" };
  if (bits < 60) return { level: 2, label: "Средний" };
  if (bits < 90) return { level: 3, label: "Надёжный" };
  return { level: 4, label: "Очень надёжный" };
}
