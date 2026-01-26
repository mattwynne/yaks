We've added migration logic (migrate_done_to_state) that runs on every yx invocation. This is fine for now, but we don't want to accumulate these migrations forever.

Two options:
1. **Database-style migrations**: Track schema version, maintain ordered migration list, apply only needed ones
2. **Time-based cleanup**: Remove migration code after a reasonable period (e.g., 6 months)

Need to decide which approach fits yaks' philosophy better. Consider:
- Yaks is a simple tool - complexity should be justified
- How often will schema change?
- What's the cost of users re-creating their .yaks if they skip versions?
- Event-sourcing approach (coming soon) might provide replay capability
