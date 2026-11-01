import type { ReactNode } from "react";
import { FieldRow } from "@/features/items/FieldRow";
import { formatDocDate, identityKindLabel } from "@/features/items/itemSchemas";
import { SecretFieldRow } from "@/features/items/SecretFieldRow";
import { useCopy } from "@/hooks/useCopy";
import type { CardView, CopyField, FieldView, IdentityView } from "@/types/vault";

interface DetailProps<T> {
  id: string;
  payload: T;
  readOnly: boolean;
}

function Card({ rows, empty }: { rows: ReactNode[]; empty: string }) {
  const visible = rows.filter(Boolean);
  return visible.length > 0 ? (
    <div className="divide-y divide-border rounded-lg border border-border bg-card">{visible}</div>
  ) : (
    <p className="text-sm text-muted-foreground">{empty}</p>
  );
}

export function NotesBlock({ notes }: { notes: string | null }) {
  if (!notes) return null;
  return (
    <div className="flex flex-col gap-1.5">
      <div className="text-xs text-muted-foreground">Заметки</div>
      <div className="rounded-lg border border-border bg-card px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap">
        {notes}
      </div>
    </div>
  );
}

/** "Свои поля": open ones are shown as is, hidden ones are revealed through Rust. */
export function CustomFieldsDetail({
  id,
  fields,
  readOnly,
}: {
  id: string;
  fields: FieldView[];
  readOnly: boolean;
}) {
  const { copyField } = useCopy();
  if (fields.length === 0) return null;
  return (
    <div className="flex flex-col gap-1.5">
      <div className="text-xs text-muted-foreground">Свои поля</div>
      <div className="divide-y divide-border rounded-lg border border-border bg-card">
        {fields.map((field, index) => {
          const name: CopyField = `custom:${index}`;
          return field.hidden ? (
            <SecretFieldRow
              key={index}
              itemId={id}
              field={name}
              label={field.label}
              readOnly={readOnly}
            />
          ) : (
            <FieldRow
              key={index}
              label={field.label}
              onCopy={readOnly ? undefined : () => void copyField(id, name, field.label)}
            >
              <span className="whitespace-pre-wrap select-text">{field.value}</span>
            </FieldRow>
          );
        })}
      </div>
    </div>
  );
}

export function CardDetail({ id, payload: card, readOnly }: DetailProps<CardView>) {
  const { copyField } = useCopy();
  const rows = [
    card.numberMasked && (
      <SecretFieldRow
        key="number"
        itemId={id}
        field="card_number"
        label="Номер карты"
        readOnly={readOnly}
        masked={<span>{card.numberMasked}</span>}
        format={(v) => v.replace(/(\d{4})(?=\d)/g, "$1 ")}
      />
    ),
    card.cardholder && (
      <FieldRow
        key="holder"
        label="Держатель"
        onCopy={readOnly ? undefined : () => void copyField(id, "cardholder")}
      >
        {card.cardholder}
      </FieldRow>
    ),
    card.expiry && (
      <FieldRow key="expiry" label="Срок действия" mono>
        {card.expiry}
      </FieldRow>
    ),
    card.hasCvv && (
      <SecretFieldRow
        key="cvv"
        itemId={id}
        field="card_cvv"
        label="CVV / CVC"
        readOnly={readOnly}
      />
    ),
    card.hasPin && (
      <SecretFieldRow key="pin" itemId={id} field="card_pin" label="PIN-код" readOnly={readOnly} />
    ),
  ];
  return (
    <div className="flex flex-col gap-5">
      <Card rows={rows} empty="В этой карте пока нет данных" />
      <CustomFieldsDetail id={id} fields={card.fields} readOnly={readOnly} />
      <NotesBlock notes={card.notes} />
    </div>
  );
}

export function IdentityDetail({ id, payload: doc, readOnly }: DetailProps<IdentityView>) {
  const issued = formatDocDate(doc.issueDate);
  const expires = formatDocDate(doc.expiryDate);
  const rows = [
    <FieldRow key="kind" label="Тип документа">
      {identityKindLabel(doc.kind)}
    </FieldRow>,
    doc.fullName && (
      <FieldRow key="name" label="ФИО">
        <span className="select-text">{doc.fullName}</span>
      </FieldRow>
    ),
    doc.hasNumber && (
      <SecretFieldRow
        key="number"
        itemId={id}
        field="doc_number"
        label="Серия и номер"
        readOnly={readOnly}
      />
    ),
    doc.issuedBy && (
      <FieldRow key="issuer" label="Кем выдан">
        <span className="whitespace-normal select-text">{doc.issuedBy}</span>
      </FieldRow>
    ),
    issued && (
      <FieldRow key="issued" label="Дата выдачи">
        {issued}
      </FieldRow>
    ),
    expires && (
      <FieldRow key="expires" label="Действует до">
        {expires}
      </FieldRow>
    ),
  ];
  return (
    <div className="flex flex-col gap-5">
      <Card rows={rows} empty="В этом документе пока нет данных" />
      <CustomFieldsDetail id={id} fields={doc.fields} readOnly={readOnly} />
      <NotesBlock notes={doc.notes} />
    </div>
  );
}
