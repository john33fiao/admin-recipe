-- Synthetic migration fixture. No version 1 application was released.
CREATE TABLE materials (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    unit TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE snapshot_outbox (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    payload TEXT NOT NULL,
    published INTEGER NOT NULL DEFAULT 0 CHECK (published IN (0, 1))
);
INSERT INTO materials (id, name, unit, created_at)
VALUES ('00000000-0000-4000-8000-000000000001', '가상 이전 재료', '장', '2026-10-24T00:00:00Z');
PRAGMA user_version = 1;
