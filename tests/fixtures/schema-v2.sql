-- Synthetic T-001 v2 data. The JSON fixture is inserted into its outbox by the test.
CREATE TABLE materials (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),
    unit TEXT NOT NULL CHECK(length(unit) BETWEEN 1 AND 20),
    created_at TEXT NOT NULL
);
CREATE TABLE snapshot_outbox (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    payload TEXT NOT NULL,
    published INTEGER NOT NULL DEFAULT 0 CHECK(published IN (0, 1))
);
CREATE TABLE creation_requests (
    request_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    unit TEXT NOT NULL,
    material_id TEXT NOT NULL UNIQUE REFERENCES materials(id)
);
INSERT INTO materials VALUES
('00000000-0000-4000-8000-000000000001', '가상 이전 같은 이름', 'g', '2026-10-24T01:00:00.000Z'),
('00000000-0000-4000-8000-000000000002', '가상 이전 같은 이름', '장', '2026-10-24T00:00:00.000Z');
INSERT INTO creation_requests VALUES
('00000000-0000-4000-8000-000000000101', '가상 이전 같은 이름', 'g', '00000000-0000-4000-8000-000000000001');
PRAGMA user_version = 2;
