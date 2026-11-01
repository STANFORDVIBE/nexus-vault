import { Controller, useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { StarIcon } from "lucide-react";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PasswordInput } from "@/components/ui/password-input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { CustomFieldsEditor } from "@/features/items/CustomFieldsEditor";
import { FolderSelect } from "@/features/items/FolderSelect";
import { FormShell } from "@/features/items/FormShell";
import {
  IDENTITY_KINDS,
  identitySchema,
  identityToPayload,
  type IdentityFormValues,
} from "@/features/items/itemSchemas";
import { Field } from "@/features/items/LoginForm";
import { saveItem } from "@/features/items/saveItem";
import { TagInput } from "@/features/items/TagInput";

interface IdentityFormProps {
  itemId?: string;
  initial: IdentityFormValues;
  initialFavorite?: boolean;
  onSaved: (id: string) => void;
  onCancel: () => void;
}

export function IdentityForm({
  itemId,
  initial,
  initialFavorite,
  onSaved,
  onCancel,
}: IdentityFormProps) {
  const {
    register,
    control,
    handleSubmit,
    setValue,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<IdentityFormValues>({
    resolver: zodResolver(identitySchema),
    defaultValues: initial,
  });

  const favorite = useWatch({ control, name: "favorite" });

  async function onSubmit(values: IdentityFormValues) {
    const id = await saveItem(
      {
        type: "identity",
        payload: identityToPayload(values),
        folderId: values.folderId,
        favorite: values.favorite,
      },
      { itemId, initialFavorite, createdMessage: "Документ сохранён" },
    );
    if (id) onSaved(id);
  }

  return (
    <FormShell
      title={itemId ? "Редактирование" : "Новый документ"}
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
      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Название" htmlFor="doc-title" error={errors.title?.message}>
          <Input
            id="doc-title"
            autoFocus
            autoComplete="off"
            placeholder="Например, Мой паспорт"
            aria-invalid={!!errors.title}
            {...register("title")}
          />
        </Field>
        <Field label="Тип документа" htmlFor="doc-kind">
          <Controller
            control={control}
            name="kind"
            render={({ field }) => {
              const known = IDENTITY_KINDS.some((k) => k.value === field.value);
              return (
                <Select value={field.value || "other"} onValueChange={field.onChange}>
                  <SelectTrigger id="doc-kind" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {!known && field.value && (
                      <SelectItem value={field.value}>{field.value}</SelectItem>
                    )}
                    {IDENTITY_KINDS.map((k) => (
                      <SelectItem key={k.value} value={k.value}>
                        {k.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              );
            }}
          />
        </Field>
      </div>

      <Field label="ФИО" htmlFor="doc-name" error={errors.fullName?.message}>
        <Input id="doc-name" autoComplete="off" {...register("fullName")} />
      </Field>

      <Field label="Серия и номер" htmlFor="doc-number" error={errors.number?.message}>
        <PasswordInput id="doc-number" aria-invalid={!!errors.number} {...register("number")} />
      </Field>

      <Field label="Кем выдан" htmlFor="doc-issuer" error={errors.issuedBy?.message}>
        <Input id="doc-issuer" autoComplete="off" {...register("issuedBy")} />
      </Field>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Дата выдачи" htmlFor="doc-issued">
          <Input id="doc-issued" type="date" {...register("issueDate")} />
        </Field>
        <Field label="Действует до" htmlFor="doc-expires">
          <Input id="doc-expires" type="date" {...register("expiryDate")} />
        </Field>
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Папка" htmlFor="doc-folder">
          <Controller
            control={control}
            name="folderId"
            render={({ field }) => (
              <FolderSelect id="doc-folder" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
        <Field label="Теги" htmlFor="doc-tags">
          <Controller
            control={control}
            name="tags"
            render={({ field }) => (
              <TagInput id="doc-tags" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
      </div>

      <Controller
        control={control}
        name="fields"
        render={({ field }) => (
          <CustomFieldsEditor idPrefix="doc" value={field.value} onChange={field.onChange} />
        )}
      />

      <Field label="Заметки" htmlFor="doc-notes" error={errors.notes?.message}>
        <Textarea id="doc-notes" rows={3} {...register("notes")} />
      </Field>
    </FormShell>
  );
}
