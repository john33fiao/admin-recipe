-- Frozen synthetic T-004 schema7 state captured through its release API.
-- Install all rows in one deferred-FK transaction; keep journal bytes unchanged.
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
CREATE TABLE material_versions(
        id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL REFERENCES materials(id),name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),unit TEXT NOT NULL CHECK(length(unit) BETWEEN 1 AND 20),canonical_unit TEXT NOT NULL,dimension TEXT NOT NULL CHECK(dimension IN('mass','volume','custom')),kind TEXT NOT NULL CHECK(kind IN('raw','prepared','stocked','menu','unclassified')),description TEXT NOT NULL,archived INTEGER NOT NULL CHECK(archived IN(0,1)),recorded_at TEXT NOT NULL,sequence INTEGER REFERENCES snapshot_outbox(sequence),source TEXT NOT NULL,UNIQUE(material_id,id));
CREATE TABLE material_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES material_versions(material_id,id));
CREATE TABLE material_prices(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
CREATE TABLE mutation_requests(request_id TEXT PRIMARY KEY NOT NULL,operation TEXT NOT NULL,payload TEXT NOT NULL,response TEXT NOT NULL,epoch TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
CREATE TABLE catalog_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),epoch TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision>=0));
CREATE TABLE business_records(
            id TEXT PRIMARY KEY NOT NULL,kind TEXT NOT NULL,
            created_at TEXT NOT NULL,created_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            corrects_confirmation_id TEXT REFERENCES business_confirmations(id));
CREATE TABLE business_revisions(
            id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL REFERENCES business_records(id),
            previous_revision_id TEXT,status TEXT NOT NULL CHECK(status IN('draft','confirmed','archived')),
            payload_version INTEGER NOT NULL CHECK(payload_version=1),payload_json TEXT NOT NULL,
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            UNIQUE(record_id,id),FOREIGN KEY(record_id,previous_revision_id) REFERENCES business_revisions(record_id,id));
CREATE TABLE business_heads(
            record_id TEXT PRIMARY KEY NOT NULL REFERENCES business_records(id),current_revision_id TEXT NOT NULL,
            FOREIGN KEY(record_id,current_revision_id) REFERENCES business_revisions(record_id,id));
CREATE TABLE business_confirmations(
            id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL UNIQUE REFERENCES business_records(id),
            revision_id TEXT NOT NULL UNIQUE,request_id TEXT NOT NULL UNIQUE REFERENCES mutation_requests(request_id) DEFERRABLE INITIALLY DEFERRED,
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            FOREIGN KEY(record_id,revision_id) REFERENCES business_revisions(record_id,id));
CREATE TABLE business_catalog_price_results(
            confirmation_id TEXT PRIMARY KEY NOT NULL REFERENCES business_confirmations(id),
            price_id TEXT NOT NULL UNIQUE REFERENCES material_prices(id));
CREATE TABLE business_corrections(
            target_confirmation_id TEXT PRIMARY KEY NOT NULL REFERENCES business_confirmations(id),
            replacement_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
            CHECK(target_confirmation_id<>replacement_confirmation_id));
CREATE TABLE purchase_orders(id TEXT PRIMARY KEY NOT NULL,first_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),created_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
CREATE TABLE "purchase_order_versions"(id TEXT PRIMARY KEY NOT NULL,order_id TEXT NOT NULL REFERENCES purchase_orders(id),previous_version_id TEXT,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),action TEXT NOT NULL,status TEXT NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,supplier TEXT,expected_date TEXT,estimated_amount_micros INTEGER CHECK(estimated_amount_micros>=0 AND estimated_amount_micros<=9000000000000000000),note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(order_id,id),FOREIGN KEY(order_id,previous_version_id) REFERENCES "purchase_order_versions"(order_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),CHECK((action IN('place','revise') AND status='open') OR (action='cancel' AND status='cancelled') OR(action='receive' AND status='received')));
CREATE TABLE "purchase_order_heads"(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),current_version_id TEXT NOT NULL,FOREIGN KEY(order_id,current_version_id) REFERENCES "purchase_order_versions"(order_id,id));
CREATE TABLE receipts(id TEXT PRIMARY KEY NOT NULL,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),receipt_date TEXT NOT NULL,note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),source_kind TEXT NOT NULL,order_id TEXT REFERENCES purchase_orders(id),ordered_version_id TEXT,FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(order_id,ordered_version_id) REFERENCES purchase_order_versions(order_id,id),CHECK((source_kind='direct' AND order_id IS NULL AND ordered_version_id IS NULL) OR (source_kind='order' AND order_id IS NOT NULL AND ordered_version_id IS NOT NULL)));
CREATE TABLE holding_versions(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),baseline_receipt_id TEXT NOT NULL REFERENCES receipts(id),baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(material_id,previous_version_id) REFERENCES holding_versions(material_id,id));
CREATE TABLE holding_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,last_valid_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES holding_versions(material_id,id),FOREIGN KEY(material_id,last_valid_version_id) REFERENCES holding_versions(material_id,id));
CREATE TABLE order_receipt_closures(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),initial_version_id TEXT NOT NULL,ordered_version_id TEXT NOT NULL,received_version_id TEXT NOT NULL UNIQUE,received_quantity TEXT NOT NULL,difference TEXT NOT NULL,FOREIGN KEY(order_id,initial_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,ordered_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,received_version_id) REFERENCES purchase_order_versions(order_id,id));
CREATE TABLE recipe_versions(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,copied_from_version_id TEXT REFERENCES recipe_versions(id),basis_count INTEGER NOT NULL CHECK(basis_count BETWEEN 1 AND 1000000000),reference_quantity_micros INTEGER CHECK(reference_quantity_micros>0 AND reference_quantity_micros<=9000000000000000),reference_unit TEXT,reference_input_quantity TEXT,reference_input_unit TEXT,selling_price_micros INTEGER CHECK(selling_price_micros>=0 AND selling_price_micros<=9000000000000000000),cooking_description TEXT NOT NULL,process TEXT NOT NULL,reference_time TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(material_id,previous_version_id) REFERENCES recipe_versions(material_id,id),CHECK((reference_quantity_micros IS NULL AND reference_unit IS NULL AND reference_input_quantity IS NULL AND reference_input_unit IS NULL) OR (reference_quantity_micros IS NOT NULL AND reference_unit IS NOT NULL AND reference_input_quantity IS NOT NULL AND reference_input_unit IS NOT NULL AND basis_count=1)),CHECK(previous_version_id IS NULL OR copied_from_version_id IS NULL));
CREATE TABLE recipe_lines(id TEXT PRIMARY KEY NOT NULL,recipe_version_id TEXT NOT NULL REFERENCES recipe_versions(id),position INTEGER NOT NULL CHECK(position>=0),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,note TEXT NOT NULL,UNIQUE(recipe_version_id,position),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
CREATE TABLE recipe_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES recipe_versions(material_id,id));
CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
CREATE INDEX receipts_material ON receipts(material_id,sequence);
CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
CREATE INDEX recipe_versions_material ON recipe_versions(material_id,sequence);
CREATE INDEX recipe_lines_material ON recipe_lines(material_id,recipe_version_id);
INSERT INTO "materials" VALUES('b830b59c-f445-4b5d-9b8c-83109cab4a84','가상 생산 곡물','g','2026-10-24T14:55:47.607Z');
INSERT INTO "materials" VALUES('5db9f582-8257-41f3-9a34-a6cc824ba551','가상 생산 메뉴','개','2026-10-24T14:55:47.857Z');
INSERT INTO "materials" VALUES('2f1910d8-650e-4e94-8c8a-424dce67880d','가상 생산 메뉴 (2)','개','2026-10-24T14:55:47.998Z');
INSERT INTO "snapshot_outbox" VALUES(1,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "1",
  "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
  "recorded_at": "2026-10-24T14:55:47.607Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "1"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      }
    ],
    "business_records": [],
    "business_revisions": [],
    "business_heads": [],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": [],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(2,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "2",
  "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
  "recorded_at": "2026-10-24T14:55:47.630Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "2"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      }
    ],
    "business_records": [],
    "business_revisions": [],
    "business_heads": [],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": [],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(3,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "3",
  "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
  "recorded_at": "2026-10-24T14:55:47.652Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "3"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8"
      }
    ],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": [],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(4,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "4",
  "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
  "recorded_at": "2026-10-24T14:55:47.701Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "4"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
      }
    ],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(5,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "5",
  "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
  "recorded_at": "2026-10-24T14:55:47.764Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "5"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
      }
    ],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(6,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "6",
  "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
  "recorded_at": "2026-10-24T14:55:47.810Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "6"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(7,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "7",
  "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
  "recorded_at": "2026-10-24T14:55:47.857Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "7"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(8,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "8",
  "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
  "recorded_at": "2026-10-24T14:55:47.905Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 설명 변경",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.905Z",
        "seq": "8",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "8"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      },
      {
        "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
        "operation": "material.revise",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"982a0de6-8d67-4bc0-a236-2a9e82d90fbc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"7\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\"}]",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"d3306307-e6cb-4db1-a1e2-cabdb8656e2d\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.905Z\",\"price\":{\"source\":\"holding_average\",\"source_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"amount\":\"30000\",\"quantity\":\"3000\",\"reason\":null},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"8\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "8"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(9,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "9",
  "request_id": "aab4e1e1-876d-47ac-a5ba-346e4819c453",
  "recorded_at": "2026-10-24T14:55:47.951Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 설명 변경",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.905Z",
        "seq": "8",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "9"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      },
      {
        "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
        "operation": "material.revise",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"982a0de6-8d67-4bc0-a236-2a9e82d90fbc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"7\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\"}]",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"d3306307-e6cb-4db1-a1e2-cabdb8656e2d\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.905Z\",\"price\":{\"source\":\"holding_average\",\"source_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"amount\":\"30000\",\"quantity\":\"3000\",\"reason\":null},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"8\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "8"
      },
      {
        "request_id": "aab4e1e1-876d-47ac-a5ba-346e4819c453",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"aab4e1e1-876d-47ac-a5ba-346e4819c453\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"8\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"120\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"3735e315-3a6c-45f9-90a2-874c7de77306\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"49df37a8-0d5d-49b1-8849-8b49f22327cf\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.951Z\",\"seq\":\"9\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"9\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "9"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      },
      {
        "id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.951Z",
        "seq": "9"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "3735e315-3a6c-45f9-90a2-874c7de77306",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "49df37a8-0d5d-49b1-8849-8b49f22327cf",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(10,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "10",
  "request_id": "952b87c3-6e0e-4981-9dad-54b89d831aaf",
  "recorded_at": "2026-10-24T14:55:47.998Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      },
      {
        "id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.998Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 설명 변경",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.905Z",
        "seq": "8",
        "source": "catalog"
      },
      {
        "id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "10"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      },
      {
        "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
        "operation": "material.revise",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"982a0de6-8d67-4bc0-a236-2a9e82d90fbc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"7\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\"}]",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"d3306307-e6cb-4db1-a1e2-cabdb8656e2d\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.905Z\",\"price\":{\"source\":\"holding_average\",\"source_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"amount\":\"30000\",\"quantity\":\"3000\",\"reason\":null},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"8\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "8"
      },
      {
        "request_id": "aab4e1e1-876d-47ac-a5ba-346e4819c453",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"aab4e1e1-876d-47ac-a5ba-346e4819c453\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"8\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"120\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"3735e315-3a6c-45f9-90a2-874c7de77306\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"49df37a8-0d5d-49b1-8849-8b49f22327cf\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.951Z\",\"seq\":\"9\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"9\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "9"
      },
      {
        "request_id": "952b87c3-6e0e-4981-9dad-54b89d831aaf",
        "operation": "recipe.copy",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"952b87c3-6e0e-4981-9dad-54b89d831aaf\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"9\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"source_recipe_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\"}]",
        "response": "{\"item\":{\"id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"name\":\"가상 생산 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.998Z\",\"updated_at\":\"2026-10-24T14:55:47.998Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8\",\"material_id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"material_version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"material_name\":\"가상 생산 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ef1fcde6-9c7f-49d4-a640-6cc61aa6903c\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"3bdffa95-fef6-45ee-9c33-692b1e696b6d\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.998Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"10\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "10"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      },
      {
        "id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.951Z",
        "seq": "9"
      },
      {
        "id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "material_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_name": "가상 생산 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "3735e315-3a6c-45f9-90a2-874c7de77306",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "49df37a8-0d5d-49b1-8849-8b49f22327cf",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "ef1fcde6-9c7f-49d4-a640-6cc61aa6903c",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "3bdffa95-fef6-45ee-9c33-692b1e696b6d",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(11,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "11",
  "request_id": "e4a95b6d-c631-4de6-a35f-5505aa8817a2",
  "recorded_at": "2026-10-24T14:55:48.061Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      },
      {
        "id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.998Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 설명 변경",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.905Z",
        "seq": "8",
        "source": "catalog"
      },
      {
        "id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "11"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      },
      {
        "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
        "operation": "material.revise",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"982a0de6-8d67-4bc0-a236-2a9e82d90fbc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"7\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\"}]",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"d3306307-e6cb-4db1-a1e2-cabdb8656e2d\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.905Z\",\"price\":{\"source\":\"holding_average\",\"source_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"amount\":\"30000\",\"quantity\":\"3000\",\"reason\":null},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"8\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "8"
      },
      {
        "request_id": "aab4e1e1-876d-47ac-a5ba-346e4819c453",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"aab4e1e1-876d-47ac-a5ba-346e4819c453\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"8\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"120\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"3735e315-3a6c-45f9-90a2-874c7de77306\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"49df37a8-0d5d-49b1-8849-8b49f22327cf\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.951Z\",\"seq\":\"9\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"9\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "9"
      },
      {
        "request_id": "952b87c3-6e0e-4981-9dad-54b89d831aaf",
        "operation": "recipe.copy",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"952b87c3-6e0e-4981-9dad-54b89d831aaf\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"9\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"source_recipe_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\"}]",
        "response": "{\"item\":{\"id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"name\":\"가상 생산 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.998Z\",\"updated_at\":\"2026-10-24T14:55:47.998Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8\",\"material_id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"material_version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"material_name\":\"가상 생산 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ef1fcde6-9c7f-49d4-a640-6cc61aa6903c\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"3bdffa95-fef6-45ee-9c33-692b1e696b6d\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.998Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"10\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "10"
      },
      {
        "request_id": "e4a95b6d-c631-4de6-a35f-5505aa8817a2",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"e4a95b6d-c631-4de6-a35f-5505aa8817a2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"150\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"31933612-bf28-499f-9521-087c4a481b91\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"150\",\"unit\":\"g\",\"input_quantity\":\"150\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1f27aba5-9746-4854-bef5-9ef654c3a57f\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:48.061Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"11\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "11"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      },
      {
        "id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.951Z",
        "seq": "9"
      },
      {
        "id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "material_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_name": "가상 생산 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10"
      },
      {
        "id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:48.061Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "3735e315-3a6c-45f9-90a2-874c7de77306",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "49df37a8-0d5d-49b1-8849-8b49f22327cf",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "ef1fcde6-9c7f-49d4-a640-6cc61aa6903c",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "3bdffa95-fef6-45ee-9c33-692b1e696b6d",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "line": {
          "id": "31933612-bf28-499f-9521-087c4a481b91",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "150",
          "unit": "g",
          "input_quantity": "150",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "line": {
          "id": "1f27aba5-9746-4854-bef5-9ef654c3a57f",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(12,'{
  "format_version": 2,
  "schema_version": 7,
  "sequence": "12",
  "request_id": "e3f223fe-07cd-4f84-84d8-ef743c8893c6",
  "recorded_at": "2026-10-24T14:55:48.138Z",
  "state": {
    "materials": [
      {
        "id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "created_at": "2026-10-24T14:55:47.607Z"
      },
      {
        "id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.857Z"
      },
      {
        "id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T14:55:47.998Z"
      }
    ],
    "material_versions": [
      {
        "id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.607Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "name": "가상 생산 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "name": "가상 생산 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 설명 변경",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.905Z",
        "seq": "8",
        "source": "catalog"
      },
      {
        "id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "name": "가상 생산 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a"
      },
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "d3306307-e6cb-4db1-a1e2-cabdb8656e2d"
      }
    ],
    "material_prices": [
      {
        "id": "a22fb1ab-f2e5-4310-a90f-5212e5adf1cb",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T14:55:47.630Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
      "revision": "12"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "5b021409-2bdf-419f-b0cc-8f238098980d",
        "operation": "material.create",
        "payload": "{\"request_id\":\"5b021409-2bdf-419f-b0cc-8f238098980d\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"0\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.607Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"1\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "1"
      },
      {
        "request_id": "2c904ace-69be-44ce-80a5-da6c4496bd7a",
        "operation": "material.price",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"2c904ace-69be-44ce-80a5-da6c4496bd7a\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"1\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T14:55:47.630Z\",\"seq\":\"2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"2\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "2"
      },
      {
        "request_id": "b8921fad-df4a-4a30-8dca-b64a2dd797ea",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b8921fad-df4a-4a30-8dca-b64a2dd797ea\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.652Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.652Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"3\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "3"
      },
      {
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "operation": "business.confirm",
        "payload": "[\"4354a998-5e15-4442-873c-bfc23739849e\",{\"meta\":{\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"kind\":\"purchase_order\",\"head_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.652Z\",\"updated_at\":\"2026-10-24T14:55:47.701Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"effective_confirmation_id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"previous_revision_id\":\"6fdce445-9b01-412e-bdb9-d4933b5af7a8\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"bc51138f-7f94-43ff-aec1-c27da08a0f8d\",\"record_id\":\"4354a998-5e15-4442-873c-bfc23739849e\",\"revision_id\":\"03b15a1c-b328-4d21-a8ac-d2c1ab85e147\",\"request_id\":\"fb1897de-336d-4e10-b752-cab086fca27b\",\"recorded_at\":\"2026-10-24T14:55:47.701Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"4\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "4"
      },
      {
        "request_id": "c402b71a-e0e9-4e5f-a8c5-41346925c297",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"c402b71a-e0e9-4e5f-a8c5-41346925c297\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.764Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.764Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"5\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "5"
      },
      {
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "operation": "business.confirm",
        "payload": "[\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",{\"meta\":{\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"5\"},\"expected_head_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"kind\":\"receipt\",\"head_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T14:55:47.764Z\",\"updated_at\":\"2026-10-24T14:55:47.810Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"effective_confirmation_id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"issue_count\":0,\"material_name\":\"가상 생산 곡물\"},\"head\":{\"id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"previous_revision_id\":\"8b9e0d04-466c-4363-a596-e2d36a6372d5\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"expected_order_version_id\":\"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45640f90-63bd-4e5a-b618-943cf82b31f6\",\"record_id\":\"87a21f4d-15f6-4bae-afea-488fe7b46d70\",\"revision_id\":\"bd992de1-f2a5-41cb-9117-926909a1615e\",\"request_id\":\"5e5a7bf3-35f2-488a-bba8-a1141787a1dc\",\"recorded_at\":\"2026-10-24T14:55:47.810Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"d56e187b-37fa-4351-a30d-04b0b7a9a4c5\",\"holding_version_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"order_id\":\"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb\",\"order_version_id\":\"45143935-cbe3-41e4-bc3d-f0d2c302ac09\"}},\"issues\":[]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"6\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "6"
      },
      {
        "request_id": "6a335065-4ad4-41b9-9e88-41be242acc1b",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"6a335065-4ad4-41b9-9e88-41be242acc1b\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"6\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"f3bf47d5-4e35-4bb0-9504-190f43c65df4\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"81b48537-2c01-4d30-9c32-1954d6c730bc\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.857Z\",\"seq\":\"7\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"7\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "7"
      },
      {
        "request_id": "982a0de6-8d67-4bc0-a236-2a9e82d90fbc",
        "operation": "material.revise",
        "payload": "[\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",{\"request_id\":\"982a0de6-8d67-4bc0-a236-2a9e82d90fbc\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"7\",\"expected_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\"}]",
        "response": "{\"id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"version_id\":\"d3306307-e6cb-4db1-a1e2-cabdb8656e2d\",\"name\":\"가상 생산 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 설명 변경\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.607Z\",\"updated_at\":\"2026-10-24T14:55:47.905Z\",\"price\":{\"source\":\"holding_average\",\"source_id\":\"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee\",\"amount\":\"30000\",\"quantity\":\"3000\",\"reason\":null},\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"8\"}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "8"
      },
      {
        "request_id": "aab4e1e1-876d-47ac-a5ba-346e4819c453",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"aab4e1e1-876d-47ac-a5ba-346e4819c453\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"8\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"120\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"4949262e-a899-4658-b3d8-21ccd4bf8b0e\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"3735e315-3a6c-45f9-90a2-874c7de77306\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"49df37a8-0d5d-49b1-8849-8b49f22327cf\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.951Z\",\"seq\":\"9\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"9\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "9"
      },
      {
        "request_id": "952b87c3-6e0e-4981-9dad-54b89d831aaf",
        "operation": "recipe.copy",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"952b87c3-6e0e-4981-9dad-54b89d831aaf\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"9\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"source_recipe_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\"}]",
        "response": "{\"item\":{\"id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"name\":\"가상 생산 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.998Z\",\"updated_at\":\"2026-10-24T14:55:47.998Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8\",\"material_id\":\"2f1910d8-650e-4e94-8c8a-424dce67880d\",\"material_version_id\":\"5f15249e-5756-4bcc-ad09-e0e62c15d711\",\"material_name\":\"가상 생산 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ef1fcde6-9c7f-49d4-a640-6cc61aa6903c\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"120\",\"unit\":\"g\",\"input_quantity\":\"120\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"3bdffa95-fef6-45ee-9c33-692b1e696b6d\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:47.998Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"10\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "10"
      },
      {
        "request_id": "e4a95b6d-c631-4de6-a35f-5505aa8817a2",
        "operation": "recipe.revise",
        "payload": "[\"5db9f582-8257-41f3-9a34-a6cc824ba551\",{\"meta\":{\"request_id\":\"e4a95b6d-c631-4de6-a35f-5505aa8817a2\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"expected_recipe_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"150\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}]",
        "response": "{\"item\":{\"id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"name\":\"가상 생산 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T14:55:47.857Z\",\"updated_at\":\"2026-10-24T14:55:47.857Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd\",\"material_id\":\"5db9f582-8257-41f3-9a34-a6cc824ba551\",\"material_version_id\":\"4e9ac84d-3602-4fba-90d9-90198fe3a68a\",\"material_name\":\"가상 생산 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":\"a3cd21f2-6522-4b53-a576-532498ac03e1\",\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"31933612-bf28-499f-9521-087c4a481b91\",\"position\":\"0\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"150\",\"unit\":\"g\",\"input_quantity\":\"150\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1f27aba5-9746-4854-bef5-9ef654c3a57f\",\"position\":\"1\",\"material_id\":\"b830b59c-f445-4b5d-9b8c-83109cab4a84\",\"material_version_id\":\"64722386-30cf-45de-a7e9-fdd126a4e8fb\",\"material_name\":\"가상 생산 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T14:55:48.061Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"11\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "11"
      },
      {
        "request_id": "e3f223fe-07cd-4f84-84d8-ef743c8893c6",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e3f223fe-07cd-4f84-84d8-ef743c8893c6\",\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입력\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4aadf3ca-d71e-4e52-b945-1e45ed518953\",\"kind\":\"receipt\",\"head_id\":\"4be9c611-c707-4e0b-997a-6d7291775423\",\"status\":\"draft\",\"created_at\":\"2026-10-24T14:55:48.138Z\",\"updated_at\":\"2026-10-24T14:55:48.138Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":5,\"material_name\":null},\"head\":{\"id\":\"4be9c611-c707-4e0b-997a-6d7291775423\",\"record_id\":\"4aadf3ca-d71e-4e52-b945-1e45ed518953\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입력\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T14:55:48.138Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[{\"field\":\"source.material_id\",\"code\":\"missing_reference\",\"message\":\"입고할 재료를 선택하세요\"},{\"field\":\"quantity\",\"code\":\"invalid_quantity\",\"message\":\"소수점 뒤의 숫자를 입력하세요\"},{\"field\":\"unit\",\"code\":\"invalid_unit\",\"message\":\"관리 단위는 제어문자 없이 1~20자로 입력하세요\"},{\"field\":\"amount\",\"code\":\"invalid_amount\",\"message\":\"0 이상의 십진수와 소수 최대 6자리로 입력하세요\"},{\"field\":\"receipt_date\",\"code\":\"invalid_date\",\"message\":\"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요\"}]},\"state\":{\"epoch\":\"66322629-e31d-49ae-9628-db8dd762437a\",\"revision\":\"12\"}}",
        "epoch": "66322629-e31d-49ae-9628-db8dd762437a",
        "sequence": "12"
      }
    ],
    "business_records": [
      {
        "id": "4354a998-5e15-4442-873c-bfc23739849e",
        "kind": "purchase_order",
        "created_at": "2026-10-24T14:55:47.652Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:47.764Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "4aadf3ca-d71e-4e52-b945-1e45ed518953",
        "kind": "receipt",
        "created_at": "2026-10-24T14:55:48.138Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.652Z",
        "seq": "3"
      },
      {
        "id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "previous_revision_id": "6fdce445-9b01-412e-bdb9-d4933b5af7a8",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
            "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.764Z",
        "seq": "5"
      },
      {
        "id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "previous_revision_id": "8b9e0d04-466c-4363-a596-e2d36a6372d5",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
              "expected_order_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
              "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb"
            },
            "quantity": "3",
            "unit": "kg",
            "amount": "30000",
            "receipt_date": "2026-10-01",
            "note": "가상 실제 입고",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      },
      {
        "id": "4be9c611-c707-4e0b-997a-6d7291775423",
        "record_id": "4aadf3ca-d71e-4e52-b945-1e45ed518953",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "direct",
              "material_id": null,
              "material_version_id": null
            },
            "quantity": "1.",
            "unit": "",
            "amount": "",
            "receipt_date": "",
            "note": "가상 미완성 입력",
            "stock_effect": {
              "kind": "unreflected"
            }
          }
        },
        "recorded_at": "2026-10-24T14:55:48.138Z",
        "seq": "12"
      }
    ],
    "business_heads": [
      {
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "current_revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147"
      },
      {
        "record_id": "4aadf3ca-d71e-4e52-b945-1e45ed518953",
        "current_revision_id": "4be9c611-c707-4e0b-997a-6d7291775423"
      },
      {
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "current_revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e"
      }
    ],
    "business_confirmations": [
      {
        "id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "revision_id": "03b15a1c-b328-4d21-a8ac-d2c1ab85e147",
        "request_id": "fb1897de-336d-4e10-b752-cab086fca27b",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "revision_id": "bd992de1-f2a5-41cb-9117-926909a1615e",
        "request_id": "5e5a7bf3-35f2-488a-bba8-a1141787a1dc",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "first_confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "created_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": null,
        "confirmation_id": "bc51138f-7f94-43ff-aec1-c27da08a0f8d",
        "business_record_id": "4354a998-5e15-4442-873c-bfc23739849e",
        "action": "place",
        "status": "open",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.701Z",
        "seq": "4"
      },
      {
        "id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "previous_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "action": "receive",
        "status": "received",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "current_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09"
      }
    ],
    "receipts": [
      {
        "id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "business_record_id": "87a21f4d-15f6-4bae-afea-488fe7b46d70",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "material_name": "가상 생산 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
          "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
        "previous_version_id": null,
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T14:55:47.810Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
        "current_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee",
        "last_valid_version_id": "cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb",
        "receipt_id": "d56e187b-37fa-4351-a30d-04b0b7a9a4c5",
        "confirmation_id": "45640f90-63bd-4e5a-b618-943cf82b31f6",
        "initial_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "ordered_version_id": "cbdfc243-c28e-47df-88c1-19bfbdd8dfeb",
        "received_version_id": "45143935-cbe3-41e4-bc3d-f0d2c302ac09",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.857Z",
        "seq": "7"
      },
      {
        "id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.951Z",
        "seq": "9"
      },
      {
        "id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "material_version_id": "5f15249e-5756-4bcc-ad09-e0e62c15d711",
        "material_name": "가상 생산 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:47.998Z",
        "seq": "10"
      },
      {
        "id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "material_version_id": "4e9ac84d-3602-4fba-90d9-90198fe3a68a",
        "material_name": "가상 생산 메뉴",
        "material_unit": "개",
        "previous_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T14:55:48.061Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "f3bf47d5-4e35-4bb0-9504-190f43c65df4",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "4949262e-a899-4658-b3d8-21ccd4bf8b0e",
        "line": {
          "id": "81b48537-2c01-4d30-9c32-1954d6c730bc",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "3735e315-3a6c-45f9-90a2-874c7de77306",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "a3cd21f2-6522-4b53-a576-532498ac03e1",
        "line": {
          "id": "49df37a8-0d5d-49b1-8849-8b49f22327cf",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "ef1fcde6-9c7f-49d4-a640-6cc61aa6903c",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "120",
          "unit": "g",
          "input_quantity": "120",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8",
        "line": {
          "id": "3bdffa95-fef6-45ee-9c33-692b1e696b6d",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "line": {
          "id": "31933612-bf28-499f-9521-087c4a481b91",
          "position": "0",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "150",
          "unit": "g",
          "input_quantity": "150",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd",
        "line": {
          "id": "1f27aba5-9746-4854-bef5-9ef654c3a57f",
          "position": "1",
          "material_id": "b830b59c-f445-4b5d-9b8c-83109cab4a84",
          "material_version_id": "64722386-30cf-45de-a7e9-fdd126a4e8fb",
          "material_name": "가상 생산 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      }
    ],
    "recipe_heads": [
      {
        "material_id": "2f1910d8-650e-4e94-8c8a-424dce67880d",
        "current_version_id": "b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8"
      },
      {
        "material_id": "5db9f582-8257-41f3-9a34-a6cc824ba551",
        "current_version_id": "fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd"
      }
    ]
  }
}
',1);
INSERT INTO "material_versions" VALUES('64722386-30cf-45de-a7e9-fdd126a4e8fb','b830b59c-f445-4b5d-9b8c-83109cab4a84','가상 생산 곡물','g','g','mass','raw','가상 이전 자료',0,'2026-10-24T14:55:47.607Z',1,'catalog');
INSERT INTO "material_versions" VALUES('4e9ac84d-3602-4fba-90d9-90198fe3a68a','5db9f582-8257-41f3-9a34-a6cc824ba551','가상 생산 메뉴','개','개','custom','menu','가상 레시피',0,'2026-10-24T14:55:47.857Z',7,'catalog');
INSERT INTO "material_versions" VALUES('d3306307-e6cb-4db1-a1e2-cabdb8656e2d','b830b59c-f445-4b5d-9b8c-83109cab4a84','가상 생산 곡물','g','g','mass','raw','가상 설명 변경',0,'2026-10-24T14:55:47.905Z',8,'catalog');
INSERT INTO "material_versions" VALUES('5f15249e-5756-4bcc-ad09-e0e62c15d711','2f1910d8-650e-4e94-8c8a-424dce67880d','가상 생산 메뉴 (2)','개','개','custom','menu','가상 레시피',0,'2026-10-24T14:55:47.998Z',10,'catalog');
INSERT INTO "material_heads" VALUES('b830b59c-f445-4b5d-9b8c-83109cab4a84','d3306307-e6cb-4db1-a1e2-cabdb8656e2d');
INSERT INTO "material_heads" VALUES('5db9f582-8257-41f3-9a34-a6cc824ba551','4e9ac84d-3602-4fba-90d9-90198fe3a68a');
INSERT INTO "material_heads" VALUES('2f1910d8-650e-4e94-8c8a-424dce67880d','5f15249e-5756-4bcc-ad09-e0e62c15d711');
INSERT INTO "material_prices" VALUES('a22fb1ab-f2e5-4310-a90f-5212e5adf1cb','b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',1000000000,9000000000,'g','1','kg','2026-10-24T14:55:47.630Z',2);
INSERT INTO "mutation_requests" VALUES('5b021409-2bdf-419f-b0cc-8f238098980d','material.create','{"request_id":"5b021409-2bdf-419f-b0cc-8f238098980d","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"0","name":"가상 생산 곡물","unit":"g","kind":"raw","description":"가상 이전 자료"}','{"id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","name":"가상 생산 곡물","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"가상 이전 자료","archived":false,"created_at":"2026-10-24T14:55:47.607Z","updated_at":"2026-10-24T14:55:47.607Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"1"}','66322629-e31d-49ae-9628-db8dd762437a',1);
INSERT INTO "mutation_requests" VALUES('2c904ace-69be-44ce-80a5-da6c4496bd7a','material.price','["b830b59c-f445-4b5d-9b8c-83109cab4a84",{"request_id":"2c904ace-69be-44ce-80a5-da6c4496bd7a","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"1","expected_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"1","amount":"9000","unit":"kg"}]','{"id":"a22fb1ab-f2e5-4310-a90f-5212e5adf1cb","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"1000","amount":"9000","unit":"g","input_quantity":"1","input_unit":"kg","recorded_at":"2026-10-24T14:55:47.630Z","seq":"2","epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"2"}','66322629-e31d-49ae-9628-db8dd762437a',2);
INSERT INTO "mutation_requests" VALUES('b8921fad-df4a-4a30-8dca-b64a2dd797ea','business.create','{"meta":{"request_id":"b8921fad-df4a-4a30-8dca-b64a2dd797ea","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"2"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"4354a998-5e15-4442-873c-bfc23739849e","kind":"purchase_order","head_id":"6fdce445-9b01-412e-bdb9-d4933b5af7a8","status":"draft","created_at":"2026-10-24T14:55:47.652Z","updated_at":"2026-10-24T14:55:47.652Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 생산 곡물"},"head":{"id":"6fdce445-9b01-412e-bdb9-d4933b5af7a8","record_id":"4354a998-5e15-4442-873c-bfc23739849e","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T14:55:47.652Z","seq":"3"},"confirmation":null,"issues":[]},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"3"}}','66322629-e31d-49ae-9628-db8dd762437a',3);
INSERT INTO "mutation_requests" VALUES('fb1897de-336d-4e10-b752-cab086fca27b','business.confirm','["4354a998-5e15-4442-873c-bfc23739849e",{"meta":{"request_id":"fb1897de-336d-4e10-b752-cab086fca27b","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"3"},"expected_head_id":"6fdce445-9b01-412e-bdb9-d4933b5af7a8"}]','{"result":{"item":{"id":"4354a998-5e15-4442-873c-bfc23739849e","kind":"purchase_order","head_id":"03b15a1c-b328-4d21-a8ac-d2c1ab85e147","status":"confirmed","created_at":"2026-10-24T14:55:47.652Z","updated_at":"2026-10-24T14:55:47.701Z","corrects_confirmation_id":null,"confirmation_id":"bc51138f-7f94-43ff-aec1-c27da08a0f8d","effective_confirmation_id":"bc51138f-7f94-43ff-aec1-c27da08a0f8d","issue_count":0,"material_name":"가상 생산 곡물"},"head":{"id":"03b15a1c-b328-4d21-a8ac-d2c1ab85e147","record_id":"4354a998-5e15-4442-873c-bfc23739849e","previous_revision_id":"6fdce445-9b01-412e-bdb9-d4933b5af7a8","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T14:55:47.701Z","seq":"4"},"confirmation":{"id":"bc51138f-7f94-43ff-aec1-c27da08a0f8d","record_id":"4354a998-5e15-4442-873c-bfc23739849e","revision_id":"03b15a1c-b328-4d21-a8ac-d2c1ab85e147","request_id":"fb1897de-336d-4e10-b752-cab086fca27b","recorded_at":"2026-10-24T14:55:47.701Z","seq":"4","result":{"kind":"purchase_order","action":"place","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"}},"issues":[]},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"4"}}','66322629-e31d-49ae-9628-db8dd762437a',4);
INSERT INTO "mutation_requests" VALUES('c402b71a-e0e9-4e5f-a8c5-41346925c297','business.create','{"meta":{"request_id":"c402b71a-e0e9-4e5f-a8c5-41346925c297","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"4"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","expected_order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"87a21f4d-15f6-4bae-afea-488fe7b46d70","kind":"receipt","head_id":"8b9e0d04-466c-4363-a596-e2d36a6372d5","status":"draft","created_at":"2026-10-24T14:55:47.764Z","updated_at":"2026-10-24T14:55:47.764Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 생산 곡물"},"head":{"id":"8b9e0d04-466c-4363-a596-e2d36a6372d5","record_id":"87a21f4d-15f6-4bae-afea-488fe7b46d70","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","expected_order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T14:55:47.764Z","seq":"5"},"confirmation":null,"issues":[]},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"5"}}','66322629-e31d-49ae-9628-db8dd762437a',5);
INSERT INTO "mutation_requests" VALUES('5e5a7bf3-35f2-488a-bba8-a1141787a1dc','business.confirm','["87a21f4d-15f6-4bae-afea-488fe7b46d70",{"meta":{"request_id":"5e5a7bf3-35f2-488a-bba8-a1141787a1dc","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"5"},"expected_head_id":"8b9e0d04-466c-4363-a596-e2d36a6372d5"}]','{"result":{"item":{"id":"87a21f4d-15f6-4bae-afea-488fe7b46d70","kind":"receipt","head_id":"bd992de1-f2a5-41cb-9117-926909a1615e","status":"confirmed","created_at":"2026-10-24T14:55:47.764Z","updated_at":"2026-10-24T14:55:47.810Z","corrects_confirmation_id":null,"confirmation_id":"45640f90-63bd-4e5a-b618-943cf82b31f6","effective_confirmation_id":"45640f90-63bd-4e5a-b618-943cf82b31f6","issue_count":0,"material_name":"가상 생산 곡물"},"head":{"id":"bd992de1-f2a5-41cb-9117-926909a1615e","record_id":"87a21f4d-15f6-4bae-afea-488fe7b46d70","previous_revision_id":"8b9e0d04-466c-4363-a596-e2d36a6372d5","status":"confirmed","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","expected_order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T14:55:47.810Z","seq":"6"},"confirmation":{"id":"45640f90-63bd-4e5a-b618-943cf82b31f6","record_id":"87a21f4d-15f6-4bae-afea-488fe7b46d70","revision_id":"bd992de1-f2a5-41cb-9117-926909a1615e","request_id":"5e5a7bf3-35f2-488a-bba8-a1141787a1dc","recorded_at":"2026-10-24T14:55:47.810Z","seq":"6","result":{"kind":"receipt","receipt_id":"d56e187b-37fa-4351-a30d-04b0b7a9a4c5","holding_version_id":"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","order_version_id":"45143935-cbe3-41e4-bc3d-f0d2c302ac09"}},"issues":[]},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"6"}}','66322629-e31d-49ae-9628-db8dd762437a',6);
INSERT INTO "mutation_requests" VALUES('6a335065-4ad4-41b9-9e88-41be242acc1b','recipe.create','{"meta":{"request_id":"6a335065-4ad4-41b9-9e88-41be242acc1b","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"6"},"owner":{"mode":"new","name":"가상 생산 메뉴","unit":"개","kind":"menu","description":"가상 레시피"},"data":{"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"100.00","unit":"g","note":""},{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"0","unit":"g","note":"가상 적당량"}]}}','{"item":{"id":"5db9f582-8257-41f3-9a34-a6cc824ba551","version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","name":"가상 생산 메뉴","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T14:55:47.857Z","updated_at":"2026-10-24T14:55:47.857Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"4949262e-a899-4658-b3d8-21ccd4bf8b0e","material_id":"5db9f582-8257-41f3-9a34-a6cc824ba551","material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","material_name":"가상 생산 메뉴","material_unit":"개","previous_version_id":null,"copied_from_version_id":null,"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"f3bf47d5-4e35-4bb0-9504-190f43c65df4","position":"0","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"100","unit":"g","input_quantity":"100.00","input_unit":"g","note":""},{"id":"81b48537-2c01-4d30-9c32-1954d6c730bc","position":"1","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T14:55:47.857Z","seq":"7"},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"7"}}','66322629-e31d-49ae-9628-db8dd762437a',7);
INSERT INTO "mutation_requests" VALUES('982a0de6-8d67-4bc0-a236-2a9e82d90fbc','material.revise','["b830b59c-f445-4b5d-9b8c-83109cab4a84",{"request_id":"982a0de6-8d67-4bc0-a236-2a9e82d90fbc","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"7","expected_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","name":"가상 생산 곡물","unit":"g","kind":"raw","description":"가상 설명 변경"}]','{"id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","version_id":"d3306307-e6cb-4db1-a1e2-cabdb8656e2d","name":"가상 생산 곡물","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"가상 설명 변경","archived":false,"created_at":"2026-10-24T14:55:47.607Z","updated_at":"2026-10-24T14:55:47.905Z","price":{"source":"holding_average","source_id":"cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee","amount":"30000","quantity":"3000","reason":null},"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"8"}','66322629-e31d-49ae-9628-db8dd762437a',8);
INSERT INTO "mutation_requests" VALUES('aab4e1e1-876d-47ac-a5ba-346e4819c453','recipe.revise','["5db9f582-8257-41f3-9a34-a6cc824ba551",{"meta":{"request_id":"aab4e1e1-876d-47ac-a5ba-346e4819c453","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"8"},"expected_material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","expected_recipe_version_id":"4949262e-a899-4658-b3d8-21ccd4bf8b0e","data":{"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"120","unit":"g","note":""},{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"0","unit":"g","note":"가상 적당량"}]}}]','{"item":{"id":"5db9f582-8257-41f3-9a34-a6cc824ba551","version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","name":"가상 생산 메뉴","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T14:55:47.857Z","updated_at":"2026-10-24T14:55:47.857Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"a3cd21f2-6522-4b53-a576-532498ac03e1","material_id":"5db9f582-8257-41f3-9a34-a6cc824ba551","material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","material_name":"가상 생산 메뉴","material_unit":"개","previous_version_id":"4949262e-a899-4658-b3d8-21ccd4bf8b0e","copied_from_version_id":null,"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"3735e315-3a6c-45f9-90a2-874c7de77306","position":"0","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"120","unit":"g","input_quantity":"120","input_unit":"g","note":""},{"id":"49df37a8-0d5d-49b1-8849-8b49f22327cf","position":"1","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T14:55:47.951Z","seq":"9"},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"9"}}','66322629-e31d-49ae-9628-db8dd762437a',9);
INSERT INTO "mutation_requests" VALUES('952b87c3-6e0e-4981-9dad-54b89d831aaf','recipe.copy','["5db9f582-8257-41f3-9a34-a6cc824ba551",{"meta":{"request_id":"952b87c3-6e0e-4981-9dad-54b89d831aaf","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"9"},"expected_material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","source_recipe_version_id":"a3cd21f2-6522-4b53-a576-532498ac03e1"}]','{"item":{"id":"2f1910d8-650e-4e94-8c8a-424dce67880d","version_id":"5f15249e-5756-4bcc-ad09-e0e62c15d711","name":"가상 생산 메뉴 (2)","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T14:55:47.998Z","updated_at":"2026-10-24T14:55:47.998Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8","material_id":"2f1910d8-650e-4e94-8c8a-424dce67880d","material_version_id":"5f15249e-5756-4bcc-ad09-e0e62c15d711","material_name":"가상 생산 메뉴 (2)","material_unit":"개","previous_version_id":null,"copied_from_version_id":"a3cd21f2-6522-4b53-a576-532498ac03e1","basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"ef1fcde6-9c7f-49d4-a640-6cc61aa6903c","position":"0","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"120","unit":"g","input_quantity":"120","input_unit":"g","note":""},{"id":"3bdffa95-fef6-45ee-9c33-692b1e696b6d","position":"1","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T14:55:47.998Z","seq":"10"},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"10"}}','66322629-e31d-49ae-9628-db8dd762437a',10);
INSERT INTO "mutation_requests" VALUES('e4a95b6d-c631-4de6-a35f-5505aa8817a2','recipe.revise','["5db9f582-8257-41f3-9a34-a6cc824ba551",{"meta":{"request_id":"e4a95b6d-c631-4de6-a35f-5505aa8817a2","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"10"},"expected_material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","expected_recipe_version_id":"a3cd21f2-6522-4b53-a576-532498ac03e1","data":{"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"150","unit":"g","note":""},{"material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"0","unit":"g","note":"가상 적당량"}]}}]','{"item":{"id":"5db9f582-8257-41f3-9a34-a6cc824ba551","version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","name":"가상 생산 메뉴","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T14:55:47.857Z","updated_at":"2026-10-24T14:55:47.857Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd","material_id":"5db9f582-8257-41f3-9a34-a6cc824ba551","material_version_id":"4e9ac84d-3602-4fba-90d9-90198fe3a68a","material_name":"가상 생산 메뉴","material_unit":"개","previous_version_id":"a3cd21f2-6522-4b53-a576-532498ac03e1","copied_from_version_id":null,"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"31933612-bf28-499f-9521-087c4a481b91","position":"0","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"150","unit":"g","input_quantity":"150","input_unit":"g","note":""},{"id":"1f27aba5-9746-4854-bef5-9ef654c3a57f","position":"1","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","material_name":"가상 생산 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T14:55:48.061Z","seq":"11"},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"11"}}','66322629-e31d-49ae-9628-db8dd762437a',11);
INSERT INTO "mutation_requests" VALUES('e3f223fe-07cd-4f84-84d8-ef743c8893c6','business.create','{"meta":{"request_id":"e3f223fe-07cd-4f84-84d8-ef743c8893c6","epoch":"66322629-e31d-49ae-9628-db8dd762437a","expected_revision":"11"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입력","stock_effect":{"kind":"unreflected"}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"4aadf3ca-d71e-4e52-b945-1e45ed518953","kind":"receipt","head_id":"4be9c611-c707-4e0b-997a-6d7291775423","status":"draft","created_at":"2026-10-24T14:55:48.138Z","updated_at":"2026-10-24T14:55:48.138Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":5,"material_name":null},"head":{"id":"4be9c611-c707-4e0b-997a-6d7291775423","record_id":"4aadf3ca-d71e-4e52-b945-1e45ed518953","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입력","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T14:55:48.138Z","seq":"12"},"confirmation":null,"issues":[{"field":"source.material_id","code":"missing_reference","message":"입고할 재료를 선택하세요"},{"field":"quantity","code":"invalid_quantity","message":"소수점 뒤의 숫자를 입력하세요"},{"field":"unit","code":"invalid_unit","message":"관리 단위는 제어문자 없이 1~20자로 입력하세요"},{"field":"amount","code":"invalid_amount","message":"0 이상의 십진수와 소수 최대 6자리로 입력하세요"},{"field":"receipt_date","code":"invalid_date","message":"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요"}]},"state":{"epoch":"66322629-e31d-49ae-9628-db8dd762437a","revision":"12"}}','66322629-e31d-49ae-9628-db8dd762437a',12);
INSERT INTO "catalog_state" VALUES(1,'66322629-e31d-49ae-9628-db8dd762437a',12);
INSERT INTO "business_records" VALUES('4354a998-5e15-4442-873c-bfc23739849e','purchase_order','2026-10-24T14:55:47.652Z',3,NULL);
INSERT INTO "business_records" VALUES('87a21f4d-15f6-4bae-afea-488fe7b46d70','receipt','2026-10-24T14:55:47.764Z',5,NULL);
INSERT INTO "business_records" VALUES('4aadf3ca-d71e-4e52-b945-1e45ed518953','receipt','2026-10-24T14:55:48.138Z',12,NULL);
INSERT INTO "business_revisions" VALUES('6fdce445-9b01-412e-bdb9-d4933b5af7a8','4354a998-5e15-4442-873c-bfc23739849e',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T14:55:47.652Z',3);
INSERT INTO "business_revisions" VALUES('03b15a1c-b328-4d21-a8ac-d2c1ab85e147','4354a998-5e15-4442-873c-bfc23739849e','6fdce445-9b01-412e-bdb9-d4933b5af7a8','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"b830b59c-f445-4b5d-9b8c-83109cab4a84","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T14:55:47.701Z',4);
INSERT INTO "business_revisions" VALUES('8b9e0d04-466c-4363-a596-e2d36a6372d5','87a21f4d-15f6-4bae-afea-488fe7b46d70',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","expected_order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T14:55:47.764Z',5);
INSERT INTO "business_revisions" VALUES('bd992de1-f2a5-41cb-9117-926909a1615e','87a21f4d-15f6-4bae-afea-488fe7b46d70','8b9e0d04-466c-4363-a596-e2d36a6372d5','confirmed',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb","expected_order_version_id":"cbdfc243-c28e-47df-88c1-19bfbdd8dfeb","material_version_id":"64722386-30cf-45de-a7e9-fdd126a4e8fb"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T14:55:47.810Z',6);
INSERT INTO "business_revisions" VALUES('4be9c611-c707-4e0b-997a-6d7291775423','4aadf3ca-d71e-4e52-b945-1e45ed518953',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입력","stock_effect":{"kind":"unreflected"}}}','2026-10-24T14:55:48.138Z',12);
INSERT INTO "business_heads" VALUES('4354a998-5e15-4442-873c-bfc23739849e','03b15a1c-b328-4d21-a8ac-d2c1ab85e147');
INSERT INTO "business_heads" VALUES('87a21f4d-15f6-4bae-afea-488fe7b46d70','bd992de1-f2a5-41cb-9117-926909a1615e');
INSERT INTO "business_heads" VALUES('4aadf3ca-d71e-4e52-b945-1e45ed518953','4be9c611-c707-4e0b-997a-6d7291775423');
INSERT INTO "business_confirmations" VALUES('bc51138f-7f94-43ff-aec1-c27da08a0f8d','4354a998-5e15-4442-873c-bfc23739849e','03b15a1c-b328-4d21-a8ac-d2c1ab85e147','fb1897de-336d-4e10-b752-cab086fca27b','2026-10-24T14:55:47.701Z',4);
INSERT INTO "business_confirmations" VALUES('45640f90-63bd-4e5a-b618-943cf82b31f6','87a21f4d-15f6-4bae-afea-488fe7b46d70','bd992de1-f2a5-41cb-9117-926909a1615e','5e5a7bf3-35f2-488a-bba8-a1141787a1dc','2026-10-24T14:55:47.810Z',6);
INSERT INTO "purchase_orders" VALUES('3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb','bc51138f-7f94-43ff-aec1-c27da08a0f8d','2026-10-24T14:55:47.701Z',4);
INSERT INTO "purchase_order_versions" VALUES('cbdfc243-c28e-47df-88c1-19bfbdd8dfeb','3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb',NULL,'bc51138f-7f94-43ff-aec1-c27da08a0f8d','place','open','b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T14:55:47.701Z',4);
INSERT INTO "purchase_order_versions" VALUES('45143935-cbe3-41e4-bc3d-f0d2c302ac09','3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb','cbdfc243-c28e-47df-88c1-19bfbdd8dfeb','45640f90-63bd-4e5a-b618-943cf82b31f6','receive','received','b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T14:55:47.810Z',6);
INSERT INTO "purchase_order_heads" VALUES('3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb','45143935-cbe3-41e4-bc3d-f0d2c302ac09');
INSERT INTO "receipts" VALUES('d56e187b-37fa-4351-a30d-04b0b7a9a4c5','45640f90-63bd-4e5a-b618-943cf82b31f6','b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',3000000000,'g','3','kg',30000000000,'2026-10-01','가상 실제 입고','2026-10-24T14:55:47.810Z',6,'order','3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb','cbdfc243-c28e-47df-88c1-19bfbdd8dfeb');
INSERT INTO "holding_versions" VALUES('cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee','b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',NULL,'d56e187b-37fa-4351-a30d-04b0b7a9a4c5','d56e187b-37fa-4351-a30d-04b0b7a9a4c5',6,3000000000,30000000000,'2026-10-24T14:55:47.810Z',6);
INSERT INTO "holding_heads" VALUES('b830b59c-f445-4b5d-9b8c-83109cab4a84','cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee','cd0c77b7-2f2e-468f-94a4-6ecb2e0ebeee');
INSERT INTO "order_receipt_closures" VALUES('3e29f4f6-5a82-49a5-b87c-5ba4f7b99afb','d56e187b-37fa-4351-a30d-04b0b7a9a4c5','45640f90-63bd-4e5a-b618-943cf82b31f6','cbdfc243-c28e-47df-88c1-19bfbdd8dfeb','cbdfc243-c28e-47df-88c1-19bfbdd8dfeb','45143935-cbe3-41e4-bc3d-f0d2c302ac09','3000','0');
INSERT INTO "recipe_versions" VALUES('4949262e-a899-4658-b3d8-21ccd4bf8b0e','5db9f582-8257-41f3-9a34-a6cc824ba551','4e9ac84d-3602-4fba-90d9-90198fe3a68a',NULL,NULL,3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T14:55:47.857Z',7);
INSERT INTO "recipe_versions" VALUES('a3cd21f2-6522-4b53-a576-532498ac03e1','5db9f582-8257-41f3-9a34-a6cc824ba551','4e9ac84d-3602-4fba-90d9-90198fe3a68a','4949262e-a899-4658-b3d8-21ccd4bf8b0e',NULL,3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T14:55:47.951Z',9);
INSERT INTO "recipe_versions" VALUES('b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8','2f1910d8-650e-4e94-8c8a-424dce67880d','5f15249e-5756-4bcc-ad09-e0e62c15d711',NULL,'a3cd21f2-6522-4b53-a576-532498ac03e1',3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T14:55:47.998Z',10);
INSERT INTO "recipe_versions" VALUES('fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd','5db9f582-8257-41f3-9a34-a6cc824ba551','4e9ac84d-3602-4fba-90d9-90198fe3a68a','a3cd21f2-6522-4b53-a576-532498ac03e1',NULL,3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T14:55:48.061Z',11);
INSERT INTO "recipe_lines" VALUES('f3bf47d5-4e35-4bb0-9504-190f43c65df4','4949262e-a899-4658-b3d8-21ccd4bf8b0e',0,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',100000000,'g','100.00','g','');
INSERT INTO "recipe_lines" VALUES('81b48537-2c01-4d30-9c32-1954d6c730bc','4949262e-a899-4658-b3d8-21ccd4bf8b0e',1,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_lines" VALUES('3735e315-3a6c-45f9-90a2-874c7de77306','a3cd21f2-6522-4b53-a576-532498ac03e1',0,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',120000000,'g','120','g','');
INSERT INTO "recipe_lines" VALUES('49df37a8-0d5d-49b1-8849-8b49f22327cf','a3cd21f2-6522-4b53-a576-532498ac03e1',1,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_lines" VALUES('ef1fcde6-9c7f-49d4-a640-6cc61aa6903c','b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8',0,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',120000000,'g','120','g','');
INSERT INTO "recipe_lines" VALUES('3bdffa95-fef6-45ee-9c33-692b1e696b6d','b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8',1,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_lines" VALUES('31933612-bf28-499f-9521-087c4a481b91','fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd',0,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',150000000,'g','150','g','');
INSERT INTO "recipe_lines" VALUES('1f27aba5-9746-4854-bef5-9ef654c3a57f','fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd',1,'b830b59c-f445-4b5d-9b8c-83109cab4a84','64722386-30cf-45de-a7e9-fdd126a4e8fb',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_heads" VALUES('5db9f582-8257-41f3-9a34-a6cc824ba551','fa00e5c6-b882-4ee4-8fa7-ad3d12b4a8fd');
INSERT INTO "recipe_heads" VALUES('2f1910d8-650e-4e94-8c8a-424dce67880d','b2c9f9da-fe42-491e-86eb-0a5f4f4b1cc8');
PRAGMA user_version=7;
