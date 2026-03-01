# 16. Reserved fields stored as hidden (dot-prefixed) files

Date: 2026-02-28

## Status

Accepted

## Context

All reserved fields in the yak directory store use hidden (dot-prefixed) file names:
`.name`, `.id`, `.state`, `.context.md`, `.created.json`, `.parent_id`. However, the
`tags` field was stored as a plain `tags` file without the dot prefix, breaking the
convention.

This inconsistency makes it harder to enumerate custom (user-defined) fields because
there is no uniform rule to distinguish reserved fields from custom ones — you need
a special case for `tags`.

## Decision

All reserved fields must be stored as hidden dotfiles. The tags field is renamed from
`tags` to `.tags`.

Legacy events in the event store that reference `field_name: "tags"` (without dot prefix)
are mapped to `.tags` during projection replay, so no event migration is needed.

## Consequences

- Consistent convention: all reserved fields are dot-prefixed, all non-dot files are
  custom fields.
- Easier to enumerate custom fields: any non-hidden file in a yak directory is
  user-defined.
- Existing event stores with `"tags"` field names continue to work via the projection
  mapping layer.
- The `.tags` field is now included in `RESERVED_FIELDS`, preventing users from
  creating a custom field called `.tags`.
