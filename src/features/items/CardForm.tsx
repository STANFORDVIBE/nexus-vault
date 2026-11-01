import { Controller, useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { StarIcon } from "lucide-react";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PasswordInput } from "@/components/ui/password-input";
import { Textarea } from "@/components/ui/textarea";
import { CustomFieldsEditor } from "@/features/items/CustomFieldsEditor";
import { FolderSelect } from "@/features/items/FolderSelect";
import { FormShell } from "@/features/items/FormShell";
import { cardSchema, cardToPayload, type CardFormValues } from "@/features/items/itemSchemas";
import { Field } from "@/features/items/LoginForm";
import { saveItem } from "@/features/items/saveItem";
import { TagInput } from "@/features/items/TagInput";

interface CardFormProps {
  itemId?: string;
  initial: CardFormValues;
  initialFavorite?: boolean;
  onSaved: (id: string) => void;
  onCancel: () => void;
}

export function CardForm({ itemId, initial, initialFavorite, onSaved, onCancel }: CardFormProps) {
  const {
    register,
    control,
    handleSubmit,
    setValue,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<CardFormValues>({ resolver: zodResolver(cardSchema), defaultValues: initial });

  const favorite = useWatch({ control, name: "favorite" });

  async function onSubmit(values: CardFormValues) {
    const id = await saveItem(
      {
        type: "card",
        payload: cardToPayload(values),
        folderId: values.folderId,
        favorite: values.favorite,
      },
      { itemId, initialFavorite, createdMessage: "Карта сохранена" },
    );
    if (id) onSaved(id);
  }

  return (
    <FormShell
      title={itemId ? "Редактирование" : "Новая карта"}
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
      <Field label="Название" htmlFor="card-title" error={errors.title?.message}>
        <Input
          id="card-title"
          autoFocus
          autoComplete="off"
          placeholder="Например, Тинькофф Black"
          aria-invalid={!!errors.title}
          {...register("title")}
        />
      </Field>

      <Field label="Номер карты" htmlFor="card-number" error={errors.number?.message}>
        <PasswordInput
          id="card-number"
          inputMode="numeric"
          placeholder="0000 0000 0000 0000"
          aria-invalid={!!errors.number}
          {...register("number")}
        />
      </Field>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Держатель" htmlFor="card-holder" error={errors.cardholder?.message}>
          <Input
            id="card-holder"
            autoComplete="off"
            spellCheck={false}
            placeholder="IVAN IVANOV"
            {...register("cardholder")}
          />
        </Field>
        <Field label="Срок действия" htmlFor="card-expiry" error={errors.expiry?.message}>
          <Input
            id="card-expiry"
            autoComplete="off"
            inputMode="numeric"
            placeholder="ММ/ГГ"
            maxLength={7}
            aria-invalid={!!errors.expiry}
            {...register("expiry")}
          />
        </Field>
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="CVV / CVC" htmlFor="card-cvv" error={errors.cvv?.message}>
          <PasswordInput
            id="card-cvv"
            inputMode="numeric"
            maxLength={4}
            aria-invalid={!!errors.cvv}
            {...register("cvv")}
          />
        </Field>
        <Field label="PIN-код" htmlFor="card-pin" error={errors.pin?.message}>
          <PasswordInput
            id="card-pin"
            inputMode="numeric"
            maxLength={12}
            aria-invalid={!!errors.pin}
            {...register("pin")}
          />
        </Field>
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <Field label="Папка" htmlFor="card-folder">
          <Controller
            control={control}
            name="folderId"
            render={({ field }) => (
              <FolderSelect id="card-folder" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
        <Field label="Теги" htmlFor="card-tags">
          <Controller
            control={control}
            name="tags"
            render={({ field }) => (
              <TagInput id="card-tags" value={field.value} onChange={field.onChange} />
            )}
          />
        </Field>
      </div>

      <Controller
        control={control}
        name="fields"
        render={({ field }) => (
          <CustomFieldsEditor idPrefix="card" value={field.value} onChange={field.onChange} />
        )}
      />

      <Field label="Заметки" htmlFor="card-notes" error={errors.notes?.message}>
        <Textarea id="card-notes" rows={3} {...register("notes")} />
      </Field>
    </FormShell>
  );
}
