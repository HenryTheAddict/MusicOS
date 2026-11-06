import type {ComponentProps} from 'react';
import {call} from './bridge';

export const metadataFields = [
  ['title', 'Title'], ['artist', 'Artist'], ['album', 'Album'],
  ['album_artist', 'Album artist'], ['genre', 'Genre'], ['year', 'Year'],
  ['disc_number', 'Disc number'], ['track_number', 'Track number'],
] as const;
export type MetadataField = typeof metadataFields[number][0];
export type MetadataValues = Record<string, unknown>;
export type MetadataDraft = Partial<Record<MetadataField, string>>;
export type MetadataPatch = NonNullable<Parameters<typeof call<'metadata_edit'>>[1]>['patch'];
export const fieldLabel = Object.fromEntries(metadataFields) as Record<MetadataField, string>;
const numericFields = new Set<MetadataField>(['year', 'disc_number', 'track_number']);
export const displayMetadata = (value: unknown) => value == null || value === '' || value === 0 ? 'Not set' : String(value);
export const songCount = (count: number) => `${count} ${count === 1 ? 'song' : 'songs'}`;

function currentValue(field: MetadataField, value: unknown) {
  return value == null || (numericFields.has(field) && value === 0) ? '' : String(value);
}

export function fieldValues(rows: MetadataValues[], field: MetadataField) {
  return [...new Set(rows.map(row => currentValue(field, row[field])))];
}

export function tagPatch(draft: MetadataDraft, rows: MetadataValues[]): MetadataPatch {
  const patch: MetadataPatch = {};
  for (const [field, label] of metadataFields) {
    if (draft[field] === undefined) continue;
    const input = draft[field].trim();
    if (fieldValues(rows, field).every(value => value === input)) continue;
    if (field === 'year' || field === 'track_number' || field === 'disc_number') {
      if (input && !/^\d{1,4}$/.test(input)) throw new Error(`${label} must be between 0 and 9999.`);
      patch[field] = input ? Number(input) : 0;
    } else {
      if (!input && (field === 'title' || field === 'artist' || field === 'album')) {
        throw new Error(`${label} needs a value.`);
      }
      patch[field] = input;
    }
  }
  return patch;
}

export default function MetadataFields({rows, fields, draft, onChange, ...fieldset}: {
  rows: MetadataValues[];
  fields: readonly MetadataField[];
  draft: MetadataDraft;
  onChange: (field: MetadataField, value: string | undefined) => void;
} & Omit<ComponentProps<'fieldset'>, 'onChange'>) {
  return <fieldset className="tag-fields" {...fieldset}>
    <legend>Current details</legend>
    {fields.map(field => {
      const values = fieldValues(rows, field);
      const mixed = values.length > 1;
      const changed = draft[field] !== undefined;
      return <div className="tag-field" key={field}>
        <label>
          <span>{fieldLabel[field]}</span>
          <input aria-label={fieldLabel[field]} value={draft[field] ?? (mixed ? '' : values[0] ?? '')}
            inputMode={numericFields.has(field) ? 'numeric' : undefined}
            maxLength={numericFields.has(field) ? 4 : 512} placeholder={mixed ? 'Mixed values' : 'Not set'}
            onChange={event => onChange(field, event.target.value)}/>
        </label>
        {changed && <button type="button" className="tag-revert" onClick={() => onChange(field, undefined)}>Revert {fieldLabel[field].toLowerCase()}</button>}
        {mixed && <details>
          <summary>See affected songs</summary>
          <ul>{rows.map(row => <li key={String(row.id)}><span>{String(row.title)}</span><span>{displayMetadata(row[field])}</span></li>)}</ul>
        </details>}
      </div>;
    })}
  </fieldset>;
}
