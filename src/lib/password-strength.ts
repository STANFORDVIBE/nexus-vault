// Rough strength estimate for the master password UI. It is guidance only:
// the backend enforces the real minimum (10 characters).

export type StrengthLevel = 0 | 1 | 2 | 3 | 4;

export interface Strength {
  level: StrengthLevel;
  label: string;
  bits: number;
}

const LABELS = ["Очень слабый", "Слабый", "Средний", "Хороший", "Отличный"] as const;

const COMMON = [
  "password",
  "qwerty",
  "123456",
  "111111",
  "admin",
  "letmein",
  "welcome",
  "iloveyou",
  "пароль",
  "йцукен",
];

function poolSize(password: string): number {
  let pool = 0;
  if (/[a-z]/.test(password)) pool += 26;
  if (/[A-Z]/.test(password)) pool += 26;
  if (/[0-9]/.test(password)) pool += 10;
  if (/[а-яё]/.test(password)) pool += 33;
  if (/[А-ЯЁ]/.test(password)) pool += 33;
  if (/[^a-zA-Z0-9а-яёА-ЯЁ]/.test(password)) pool += 33;
  return Math.max(pool, 1);
}

export function estimateStrength(password: string): Strength {
  if (!password) return { level: 0, label: LABELS[0], bits: 0 };

  const chars = [...password];
  // repeated characters add little: count distinct ones at full weight, the rest at a quarter
  const distinct = new Set(chars).size;
  const effectiveLength = distinct + (chars.length - distinct) * 0.25;
  let bits = effectiveLength * Math.log2(poolSize(password));

  const lower = password.toLowerCase();
  if (COMMON.some((word) => lower.includes(word))) bits *= 0.5;
  if (/^(.)\1+$/.test(password)) bits = Math.min(bits, 8);

  const level: StrengthLevel =
    chars.length < 10 ? (bits < 28 ? 0 : 1) : bits < 45 ? 1 : bits < 60 ? 2 : bits < 80 ? 3 : 4;

  return { level, label: LABELS[level], bits: Math.round(bits) };
}
