import { useState } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { DicesIcon, StarIcon } from "lucide-react";
import { toast } from "sonner";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Textarea } from "@/components/ui/textarea";
import { GeneratorPanel } from "@/features/generator/GeneratorPanel";
import { CustomFieldsEditor } from "@/features/items/CustomFieldsEditor";
import { FolderSelect } from "@/features/items/FolderSelect";
import { FormShell } from "@/features/items/FormShell";
import { cleanFields } from "@/features/items/itemSchemas";
import { loginSchema, type LoginFormValues } from "@/features/items/loginSchema";
import { TagInput } from "@/features/items/TagInput";
import { errorMessage } from "@/services/errors";
import { createItem, setFavorite, updateItem } from "@/services/vault";
import { invalidate } from "@/store/session";
import type { ItemInput } from "@/types/vault";

function toInput(values: LoginFormValues): ItemInput {
  const opt = (v: string) => (v.trim() ? v.trim() : null);
  return {
    type: "login",
    payload: {
      title: values.title.trim(),
      website: opt(values.website),
      username: opt(values.username),
      email: opt(values.email),
      // passwords are stored verbatim: leading/trailing spaces may be intentional
      password: values.password ? values.password : null,
      totp: opt(values.totp),
      notes: opt(values.notes),
      tags: values.tags,
      fields: cleanFields(values.fields),
    },
    folderId: values.folderId,
    favorite: values.favorite,
  };
}

interface LoginFormProps {
  /** Present when editing an existing item. */
  itemId?: string;
  initial: LoginFormValues;
  initialFavorite?: boolean;
  onSaved: (id: string) => void;
  onCancel: () => void;
}

export function LoginForm({ itemId, initial, initialFavorite, onSaved, onCancel }: LoginFormProps) {
  const [generatorOpen, setGeneratorOpen] = useState(false);
  const {
    register,
    control,
    handleSubmit,
    setValue,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<LoginFormValues>({ resolver: zodResolver(loginSchema), defaultValues: initial });

  const favorite = useWatch({ control, name: "favorite" });

  async function onSubmit(values: LoginFormValues) {
    try {
      const input = toInput(values);
      if (itemId) {
        await updateItem(itemId, input);
        if (values.favorite !== initialFavorite) await setFavorite(itemId, values.favorite);
        invalidate();
        toast.success("Изменения сохранены");
        onSaved(itemId);
      } else {
        const id = await createItem(input);
        invalidate();
        toast.success("Пароль сохранён");
        onSaved(id);
      }
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <FormShell
      title={itemId ? "Редактирование" : "Новый пароль"}
      dirty={isDirty}
      busy={isSubmitting}
      onSubmit={handleSubmit(onSubmit)}
      onCancel={onCancel}
      headerAction={
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={favorite ? "Убрать из избранного" : "В избранное"}
          aria-pressed={favorite}
          onClick={() => setValue("favorite", !favorite, { shouldDirty: true })}
        >
          <StarIcon className={cn(favorite && "fill-brand text-brand")} />
        </Button>
      }
    >
      <Field label="Название" htmlFor="login-title" error={errors.title?.message}>
        <Input
          id="login-title"
          autoFocus
          autoComplete="off"
          placeholder="Например, GitHub"
          aria-invalid={!!errors.title}
          {...register("title")}
        />
      </Field>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Логин" htmlFor="login-username" error={errors.username?.message}>
          <Input
            id="login-username"
            autoComplete="off"
            spellCheck={false}
            {...register("username")}
          />
        </Field>
        <Field label="Email" htmlFor="login-email" error={errors.email?.message}>
          <Input
            id="login-email"
            autoComplete="off"
            spellCheck={false}
            aria-invalid={!!errors.email}
            {...register("email")}
          />
        </Field>
      </div>

      <Field label="Пароль" htmlFor="login-password" error={errors.password?.message}>
        <div className="flex gap-2">
          <div className="flex-1">
            <PasswordInput id="login-password" {...register("password")} />
          </div>
          <Popover open={generatorOpen} onOpenChange={setGeneratorOpen}>
            <PopoverTrigger asChild>
              <Button type="button" variant="outline">
                <DicesIcon />
                Сгенерировать
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-[340px] p-4">
              <GeneratorPanel
                compact
                renderActions={(value) => (
                  <Button
                    type="button"
                    disabled={!value}
                    onClick={() => {
                      setValue("password", value, { shouldDirty: true, shouldValidate: true });
                      setGeneratorOpen(false);
                    }}
                  >
                    Использовать
                  </Button>
                )}
              />
            </PopoverContent>
          </Popover>
        </div>
      </Field>

      <Field label="Ключ 2FA (необязательно)" htmlFor="login-totp" error={errors.totp?.message}>
        <PasswordInput
          id="login-totp"
          placeholder="Секретный ключ или ссылка otpauth://"
          aria-invalid={!!errors.totp}
          {...register("totp")}
        />
      </Field>

      <Field label="Сайт" htmlFor="login-website" error={errors.website?.message}>
        <Input
          id="login-website"
          autoComplete="off"
          spellCheck={false}
          placeholder="https://"
          {...register("website")}
        />
      </Field>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Папка" htmlFor="login-folder">
          <Controller
            control={control}
            name="folderId"
            render={({ field }) => (
              <FolderSelect id="login-folder" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
        <Field label="Теги" htmlFor="login-tags">
          <Controller
            control={control}
            name="tags"
            render={({ field }) => (
              <TagInput id="login-tags" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
      </div>

      <Controller
        control={control}
        name="fields"
        render={({ field }) => (
          <CustomFieldsEditor idPrefix="login" value={field.value} onChange={field.onChange} />
        )}
      />

      <Field label="Заметки" htmlFor="login-notes" error={errors.notes?.message}>
        <Textarea id="login-notes" rows={4} {...register("notes")} />
      </Field>
    </FormShell>
  );
}

export function Field({
  label,
  htmlFor,
  error,
  children,
}: {
  label: string;
  htmlFor: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
