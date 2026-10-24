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

-- Synthetic frozen catalog schema v3; outbox payloads are supplied by the test.
CREATE TABLE material_versions(
        id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL REFERENCES materials(id),name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),unit TEXT NOT NULL CHECK(length(unit) BETWEEN 1 AND 20),canonical_unit TEXT NOT NULL,dimension TEXT NOT NULL CHECK(dimension IN('mass','volume','custom')),kind TEXT NOT NULL CHECK(kind IN('raw','prepared','stocked','menu','unclassified')),description TEXT NOT NULL,archived INTEGER NOT NULL CHECK(archived IN(0,1)),recorded_at TEXT NOT NULL,sequence INTEGER REFERENCES snapshot_outbox(sequence),source TEXT NOT NULL,UNIQUE(material_id,id));
        CREATE TABLE material_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE material_prices(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE mutation_requests(request_id TEXT PRIMARY KEY NOT NULL,operation TEXT NOT NULL,payload TEXT NOT NULL,response TEXT NOT NULL,epoch TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
        CREATE TABLE catalog_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),epoch TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision>=0));
        CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
        CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
INSERT INTO catalog_state VALUES(1,'00000000-0000-4000-8000-000000000501',2);
INSERT INTO material_versions VALUES
('00000000-0000-4000-8000-000000000201','00000000-0000-4000-8000-000000000001','가상 이전 같은 이름','g','g','mass','unclassified','',0,'2026-10-24T01:00:00.000Z',1,'legacy_snapshot'),
('00000000-0000-4000-8000-000000000202','00000000-0000-4000-8000-000000000002','가상 이전 같은 이름','장','장','custom','unclassified','',0,'2026-10-24T00:00:00.000Z',NULL,'legacy_migration');
INSERT INTO material_heads VALUES
('00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000201'),
('00000000-0000-4000-8000-000000000002','00000000-0000-4000-8000-000000000202');
INSERT INTO material_prices VALUES
('00000000-0000-4000-8000-000000000301','00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000201',1000000000,10000000,'g','1','kg','2026-10-24T02:00:00.000Z',2);
PRAGMA user_version = 3;
