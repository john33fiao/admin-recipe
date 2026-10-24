-- Frozen synthetic T-008 schema8 state captured through its release API.
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
CREATE TABLE order_receipt_closures(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),initial_version_id TEXT NOT NULL,ordered_version_id TEXT NOT NULL,received_version_id TEXT NOT NULL UNIQUE,received_quantity TEXT NOT NULL,difference TEXT NOT NULL,FOREIGN KEY(order_id,initial_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,ordered_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,received_version_id) REFERENCES purchase_order_versions(order_id,id));
CREATE TABLE recipe_versions(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,copied_from_version_id TEXT REFERENCES recipe_versions(id),basis_count INTEGER NOT NULL CHECK(basis_count BETWEEN 1 AND 1000000000),reference_quantity_micros INTEGER CHECK(reference_quantity_micros>0 AND reference_quantity_micros<=9000000000000000),reference_unit TEXT,reference_input_quantity TEXT,reference_input_unit TEXT,selling_price_micros INTEGER CHECK(selling_price_micros>=0 AND selling_price_micros<=9000000000000000000),cooking_description TEXT NOT NULL,process TEXT NOT NULL,reference_time TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(material_id,previous_version_id) REFERENCES recipe_versions(material_id,id),CHECK((reference_quantity_micros IS NULL AND reference_unit IS NULL AND reference_input_quantity IS NULL AND reference_input_unit IS NULL) OR (reference_quantity_micros IS NOT NULL AND reference_unit IS NOT NULL AND reference_input_quantity IS NOT NULL AND reference_input_unit IS NOT NULL AND basis_count=1)),CHECK(previous_version_id IS NULL OR copied_from_version_id IS NULL));
CREATE TABLE recipe_lines(id TEXT PRIMARY KEY NOT NULL,recipe_version_id TEXT NOT NULL REFERENCES recipe_versions(id),position INTEGER NOT NULL CHECK(position>=0),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,note TEXT NOT NULL,UNIQUE(recipe_version_id,position),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
CREATE TABLE recipe_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES recipe_versions(material_id,id));
CREATE TABLE productions(
            id TEXT PRIMARY KEY NOT NULL,
            confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
            note TEXT NOT NULL,
            total_input_value_micros INTEGER NOT NULL CHECK(total_input_value_micros>=0 AND total_input_value_micros<=9000000000000000000),
            recorded_at TEXT NOT NULL,
            sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence));
CREATE TABLE production_effects(
            id TEXT PRIMARY KEY NOT NULL,
            production_id TEXT NOT NULL REFERENCES productions(id),
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            input_quantity_micros INTEGER NOT NULL CHECK(input_quantity_micros>=0 AND input_quantity_micros<=9000000000000000),
            input_value_micros INTEGER NOT NULL CHECK(input_value_micros>=0 AND input_value_micros<=9000000000000000000),
            output_quantity_micros INTEGER NOT NULL CHECK(output_quantity_micros>=0 AND output_quantity_micros<=9000000000000000),
            output_value_micros INTEGER NOT NULL CHECK(output_value_micros>=0 AND output_value_micros<=9000000000000000000),
            UNIQUE(production_id,material_id),
            UNIQUE(material_id,id),
            UNIQUE(production_id,material_id,material_version_id,id),
            FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
            CHECK(input_quantity_micros>0 OR output_quantity_micros>0),
            CHECK(input_quantity_micros>0 OR input_value_micros=0),
            CHECK(output_quantity_micros>0 OR output_value_micros=0));
CREATE TABLE production_inputs(
            production_id TEXT NOT NULL REFERENCES productions(id),
            position INTEGER NOT NULL CHECK(position>=0 AND position<100),
            effect_id TEXT NOT NULL,
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
            unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            PRIMARY KEY(production_id,position),
            FOREIGN KEY(production_id,material_id,material_version_id,effect_id) REFERENCES production_effects(production_id,material_id,material_version_id,id));
CREATE TABLE production_outputs(
            production_id TEXT NOT NULL REFERENCES productions(id),
            position INTEGER NOT NULL CHECK(position>=0 AND position<100),
            effect_id TEXT NOT NULL,
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
            unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            allocation_micros INTEGER NOT NULL CHECK(allocation_micros>=0 AND allocation_micros<=100000000),
            PRIMARY KEY(production_id,position),
            FOREIGN KEY(production_id,material_id,material_version_id,effect_id) REFERENCES production_effects(production_id,material_id,material_version_id,id));
CREATE TABLE "holding_versions"(
            id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,
            receipt_id TEXT UNIQUE REFERENCES receipts(id),
            baseline_receipt_id TEXT REFERENCES receipts(id),
            baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            production_effect_id TEXT UNIQUE,baseline_production_effect_id TEXT,
            UNIQUE(material_id,id),
            FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
            FOREIGN KEY(material_id,previous_version_id) REFERENCES "holding_versions"(material_id,id),
            FOREIGN KEY(material_id,production_effect_id) REFERENCES production_effects(material_id,id),
            FOREIGN KEY(material_id,baseline_production_effect_id) REFERENCES production_effects(material_id,id),
            CHECK((receipt_id IS NULL)<>(production_effect_id IS NULL)),
            CHECK((baseline_receipt_id IS NULL)<>(baseline_production_effect_id IS NULL)),
            CHECK(quantity_micros>0 OR value_micros=0));
CREATE TABLE "holding_heads"(
            material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),
            current_version_id TEXT NOT NULL,last_valid_version_id TEXT NOT NULL,
            FOREIGN KEY(material_id,current_version_id) REFERENCES "holding_versions"(material_id,id),
            FOREIGN KEY(material_id,last_valid_version_id) REFERENCES "holding_versions"(material_id,id));
CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
CREATE INDEX receipts_material ON receipts(material_id,sequence);
CREATE INDEX recipe_versions_material ON recipe_versions(material_id,sequence);
CREATE INDEX recipe_lines_material ON recipe_lines(material_id,recipe_version_id);
CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
CREATE INDEX production_effects_material ON production_effects(material_id,production_id);
INSERT INTO "materials" VALUES('1597a96e-0e12-40c6-a696-481f491e49eb','가상 판매 곡물','g','2026-10-24T15:44:03.064Z');
INSERT INTO "materials" VALUES('21edab83-b27b-4245-9192-af759cc3d5d6','가상 판매 비축품','g','2026-10-24T15:44:03.325Z');
INSERT INTO "materials" VALUES('2380fd77-6146-4527-930b-9146dd227347','가상 판매 메뉴','개','2026-10-24T15:44:03.446Z');
INSERT INTO "materials" VALUES('310b632e-ded4-4006-b5b8-2ea6f8a48916','가상 판매 메뉴 (2)','개','2026-10-24T15:44:03.507Z');
INSERT INTO "snapshot_outbox" VALUES(1,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "1",
  "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
  "recorded_at": "2026-10-24T15:44:03.064Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "1"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
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
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(2,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "2",
  "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
  "recorded_at": "2026-10-24T15:44:03.095Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "2"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
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
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(3,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "3",
  "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
  "recorded_at": "2026-10-24T15:44:03.126Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "3"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994"
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
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(4,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "4",
  "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
  "recorded_at": "2026-10-24T15:44:03.172Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "4"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
      }
    ],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(5,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "5",
  "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
  "recorded_at": "2026-10-24T15:44:03.234Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "5"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
      }
    ],
    "receipts": [],
    "holding_versions": [],
    "holding_heads": [],
    "order_receipt_closures": [],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(6,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "6",
  "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
  "recorded_at": "2026-10-24T15:44:03.278Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "6"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(7,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "7",
  "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
  "recorded_at": "2026-10-24T15:44:03.325Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "7"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(8,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "8",
  "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
  "recorded_at": "2026-10-24T15:44:03.357Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "8"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [],
    "production_inputs": [],
    "production_outputs": [],
    "production_effects": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(9,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "9",
  "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
  "recorded_at": "2026-10-24T15:44:03.401Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "9"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      },
      {
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "operation": "business.confirm",
        "payload": "[\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",{\"meta\":{\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"8\"},\"expected_head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.401Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"effective_confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"revision_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"eca709b5-6102-4989-a707-06875ed6452c\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"9\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "9"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      },
      {
        "id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "receipt_id": null,
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "last_valid_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [
      {
        "id": "eca709b5-6102-4989-a707-06875ed6452c",
        "confirmation_id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "business_record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "row": {
          "position": "0",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "3000",
          "unit": "g",
          "input_quantity": "3.00",
          "input_unit": "kg",
          "value": "30000"
        }
      }
    ],
    "production_outputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "row": {
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "2000",
          "unit": "g",
          "input_quantity": "2",
          "input_unit": "kg",
          "value": "30000",
          "allocation_percent": "100"
        }
      }
    ],
    "production_effects": [
      {
        "id": "61c15365-446a-42e6-839e-6930a7667783",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(10,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "10",
  "request_id": "8aaa3c1f-380a-470a-9a61-644a09609ed5",
  "recorded_at": "2026-10-24T15:44:03.446Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      },
      {
        "id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.446Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      },
      {
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "10"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      },
      {
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "operation": "business.confirm",
        "payload": "[\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",{\"meta\":{\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"8\"},\"expected_head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.401Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"effective_confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"revision_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"eca709b5-6102-4989-a707-06875ed6452c\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"9\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "9"
      },
      {
        "request_id": "8aaa3c1f-380a-470a-9a61-644a09609ed5",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"8aaa3c1f-380a-470a-9a61-644a09609ed5\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.446Z\",\"updated_at\":\"2026-10-24T15:44:03.446Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"material_id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ec23dc5d-e8fb-408c-89e0-901ebd21705d\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1c4fe992-93f2-46e0-b931-54e13e2fe6cc\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.446Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"10\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "10"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      },
      {
        "id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "receipt_id": null,
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "last_valid_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "material_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_name": "가상 판매 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "ec23dc5d-e8fb-408c-89e0-901ebd21705d",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "1c4fe992-93f2-46e0-b931-54e13e2fe6cc",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
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
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1"
      }
    ],
    "productions": [
      {
        "id": "eca709b5-6102-4989-a707-06875ed6452c",
        "confirmation_id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "business_record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "row": {
          "position": "0",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "3000",
          "unit": "g",
          "input_quantity": "3.00",
          "input_unit": "kg",
          "value": "30000"
        }
      }
    ],
    "production_outputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "row": {
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "2000",
          "unit": "g",
          "input_quantity": "2",
          "input_unit": "kg",
          "value": "30000",
          "allocation_percent": "100"
        }
      }
    ],
    "production_effects": [
      {
        "id": "61c15365-446a-42e6-839e-6930a7667783",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(11,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "11",
  "request_id": "8281d185-dd20-43ae-a379-d265e1a0d8b1",
  "recorded_at": "2026-10-24T15:44:03.507Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      },
      {
        "id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.446Z"
      },
      {
        "id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.507Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      },
      {
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "11"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      },
      {
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "operation": "business.confirm",
        "payload": "[\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",{\"meta\":{\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"8\"},\"expected_head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.401Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"effective_confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"revision_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"eca709b5-6102-4989-a707-06875ed6452c\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"9\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "9"
      },
      {
        "request_id": "8aaa3c1f-380a-470a-9a61-644a09609ed5",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"8aaa3c1f-380a-470a-9a61-644a09609ed5\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.446Z\",\"updated_at\":\"2026-10-24T15:44:03.446Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"material_id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ec23dc5d-e8fb-408c-89e0-901ebd21705d\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1c4fe992-93f2-46e0-b931-54e13e2fe6cc\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.446Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"10\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "10"
      },
      {
        "request_id": "8281d185-dd20-43ae-a379-d265e1a0d8b1",
        "operation": "recipe.copy",
        "payload": "[\"2380fd77-6146-4527-930b-9146dd227347\",{\"meta\":{\"request_id\":\"8281d185-dd20-43ae-a379-d265e1a0d8b1\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"source_recipe_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\"}]",
        "response": "{\"item\":{\"id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.507Z\",\"updated_at\":\"2026-10-24T15:44:03.507Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"de430f1d-d200-49d2-a105-d6eae2789fc1\",\"material_id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"material_version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"136240c0-08f2-42da-9428-a3c13d393efb\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"9d7016fe-c82e-4782-9633-67154a20e5b7\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.507Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"11\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "11"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      },
      {
        "id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "receipt_id": null,
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "last_valid_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "material_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_name": "가상 판매 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10"
      },
      {
        "id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "material_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "ec23dc5d-e8fb-408c-89e0-901ebd21705d",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "1c4fe992-93f2-46e0-b931-54e13e2fe6cc",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "136240c0-08f2-42da-9428-a3c13d393efb",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "9d7016fe-c82e-4782-9633-67154a20e5b7",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
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
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1"
      }
    ],
    "productions": [
      {
        "id": "eca709b5-6102-4989-a707-06875ed6452c",
        "confirmation_id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "business_record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "row": {
          "position": "0",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "3000",
          "unit": "g",
          "input_quantity": "3.00",
          "input_unit": "kg",
          "value": "30000"
        }
      }
    ],
    "production_outputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "row": {
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "2000",
          "unit": "g",
          "input_quantity": "2",
          "input_unit": "kg",
          "value": "30000",
          "allocation_percent": "100"
        }
      }
    ],
    "production_effects": [
      {
        "id": "61c15365-446a-42e6-839e-6930a7667783",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(12,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "12",
  "request_id": "0969c45f-7d37-4d86-8733-eae0b7609ea0",
  "recorded_at": "2026-10-24T15:44:03.552Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      },
      {
        "id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.446Z"
      },
      {
        "id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.507Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      },
      {
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "12"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      },
      {
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "operation": "business.confirm",
        "payload": "[\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",{\"meta\":{\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"8\"},\"expected_head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.401Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"effective_confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"revision_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"eca709b5-6102-4989-a707-06875ed6452c\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"9\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "9"
      },
      {
        "request_id": "8aaa3c1f-380a-470a-9a61-644a09609ed5",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"8aaa3c1f-380a-470a-9a61-644a09609ed5\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.446Z\",\"updated_at\":\"2026-10-24T15:44:03.446Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"material_id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ec23dc5d-e8fb-408c-89e0-901ebd21705d\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1c4fe992-93f2-46e0-b931-54e13e2fe6cc\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.446Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"10\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "10"
      },
      {
        "request_id": "8281d185-dd20-43ae-a379-d265e1a0d8b1",
        "operation": "recipe.copy",
        "payload": "[\"2380fd77-6146-4527-930b-9146dd227347\",{\"meta\":{\"request_id\":\"8281d185-dd20-43ae-a379-d265e1a0d8b1\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"source_recipe_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\"}]",
        "response": "{\"item\":{\"id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.507Z\",\"updated_at\":\"2026-10-24T15:44:03.507Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"de430f1d-d200-49d2-a105-d6eae2789fc1\",\"material_id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"material_version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"136240c0-08f2-42da-9428-a3c13d393efb\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"9d7016fe-c82e-4782-9633-67154a20e5b7\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.507Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"11\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "11"
      },
      {
        "request_id": "0969c45f-7d37-4d86-8733-eae0b7609ea0",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"0969c45f-7d37-4d86-8733-eae0b7609ea0\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"9349d45d-6af3-4d0f-bf6c-536eed0c369a\",\"kind\":\"production\",\"head_id\":\"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.552Z\",\"updated_at\":\"2026-10-24T15:44:03.552Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":1,\"material_name\":null},\"head\":{\"id\":\"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9\",\"record_id\":\"9349d45d-6af3-4d0f-bf6c-536eed0c369a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.552Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[{\"field\":\"outputs.0.material_id\",\"code\":\"missing_reference\",\"message\":\"생산의 모든 행에서 품목을 선택하세요\"}]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"12\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "12"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.552Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      },
      {
        "id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "1436d9e5-2239-43ba-b6e8-a8415e4d8fd9",
        "record_id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "1.",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": null,
                "material_version_id": null,
                "quantity": "",
                "unit": "",
                "allocation_percent": ""
              }
            ],
            "note": "가상 미완성 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.552Z",
        "seq": "12"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "current_revision_id": "1436d9e5-2239-43ba-b6e8-a8415e4d8fd9"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "receipt_id": null,
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "last_valid_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "material_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_name": "가상 판매 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10"
      },
      {
        "id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "material_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "ec23dc5d-e8fb-408c-89e0-901ebd21705d",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "1c4fe992-93f2-46e0-b931-54e13e2fe6cc",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "136240c0-08f2-42da-9428-a3c13d393efb",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "9d7016fe-c82e-4782-9633-67154a20e5b7",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
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
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1"
      }
    ],
    "productions": [
      {
        "id": "eca709b5-6102-4989-a707-06875ed6452c",
        "confirmation_id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "business_record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "row": {
          "position": "0",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "3000",
          "unit": "g",
          "input_quantity": "3.00",
          "input_unit": "kg",
          "value": "30000"
        }
      }
    ],
    "production_outputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "row": {
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "2000",
          "unit": "g",
          "input_quantity": "2",
          "input_unit": "kg",
          "value": "30000",
          "allocation_percent": "100"
        }
      }
    ],
    "production_effects": [
      {
        "id": "61c15365-446a-42e6-839e-6930a7667783",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(13,'{
  "format_version": 2,
  "schema_version": 8,
  "sequence": "13",
  "request_id": "a73d831d-975c-41f3-b93b-20ed1065db91",
  "recorded_at": "2026-10-24T15:44:03.584Z",
  "state": {
    "materials": [
      {
        "id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.064Z"
      },
      {
        "id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T15:44:03.325Z"
      },
      {
        "id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.446Z"
      },
      {
        "id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T15:44:03.507Z"
      }
    ],
    "material_versions": [
      {
        "id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.064Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.325Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea"
      },
      {
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d"
      }
    ],
    "material_prices": [
      {
        "id": "b4341c6d-3e62-4e71-853c-a4924153ffb5",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T15:44:03.095Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
      "revision": "13"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "837db76a-bf4c-4a33-a1cc-11b2064f4aaa",
        "operation": "material.create",
        "payload": "{\"request_id\":\"837db76a-bf4c-4a33-a1cc-11b2064f4aaa\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.064Z\",\"updated_at\":\"2026-10-24T15:44:03.064Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"1\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "1"
      },
      {
        "request_id": "5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6",
        "operation": "material.price",
        "payload": "[\"1597a96e-0e12-40c6-a696-481f491e49eb\",{\"request_id\":\"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"1\",\"expected_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"b4341c6d-3e62-4e71-853c-a4924153ffb5\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T15:44:03.095Z\",\"seq\":\"2\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"2\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "2"
      },
      {
        "request_id": "e8378d48-03dc-45db-a38f-25477b854b80",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e8378d48-03dc-45db-a38f-25477b854b80\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.126Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.126Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"3\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "3"
      },
      {
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "operation": "business.confirm",
        "payload": "[\"6de68baf-e40c-4022-9abe-3c2488e846cf\",{\"meta\":{\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"3\"},\"expected_head_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"kind\":\"purchase_order\",\"head_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.126Z\",\"updated_at\":\"2026-10-24T15:44:03.172Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"effective_confirmation_id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"previous_revision_id\":\"6e677cd5-0cb9-4e4c-a109-da922fa71994\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"aa2ff928-ba6c-47f0-995c-7c08c71396ac\",\"record_id\":\"6de68baf-e40c-4022-9abe-3c2488e846cf\",\"revision_id\":\"5804b6c7-6275-4a59-8f04-d81935269804\",\"request_id\":\"b41f14d4-0178-42aa-ad48-0b56b02ea27b\",\"recorded_at\":\"2026-10-24T15:44:03.172Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"4\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "4"
      },
      {
        "request_id": "314ef744-b77d-44b6-959e-96a06bc8aed3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"314ef744-b77d-44b6-959e-96a06bc8aed3\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.234Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.234Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"5\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "5"
      },
      {
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "operation": "business.confirm",
        "payload": "[\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",{\"meta\":{\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"5\"},\"expected_head_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"kind\":\"receipt\",\"head_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.234Z\",\"updated_at\":\"2026-10-24T15:44:03.278Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"effective_confirmation_id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"previous_revision_id\":\"bc9264f9-f153-4725-beda-a3b03223e464\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"expected_order_version_id\":\"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294\",\"record_id\":\"f1b82e50-afa2-42f1-9179-2c77eda8c8b2\",\"revision_id\":\"121f07fd-8963-4592-b446-1bf66b7d53c2\",\"request_id\":\"15964307-a9f4-45cb-a3fd-768b20299bca\",\"recorded_at\":\"2026-10-24T15:44:03.278Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"7b816f0b-3317-4fa4-8b00-1daecc37acef\",\"holding_version_id\":\"73802c25-1db1-4ef9-ad91-8be379656b9b\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"order_id\":\"0e6ba81f-cfa8-4315-b37a-6fcfaf416672\",\"order_version_id\":\"b40aec9c-e468-44f3-a588-5905ac0b99ad\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"6\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "6"
      },
      {
        "request_id": "f159c8bb-07ca-4551-94f2-a4a8015d98be",
        "operation": "material.create",
        "payload": "{\"request_id\":\"f159c8bb-07ca-4551-94f2-a4a8015d98be\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.325Z\",\"updated_at\":\"2026-10-24T15:44:03.325Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"7\"}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "7"
      },
      {
        "request_id": "9f429bd6-c535-4103-932d-e2ce4878bf0a",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"9f429bd6-c535-4103-932d-e2ce4878bf0a\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.357Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.357Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"8\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "8"
      },
      {
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "operation": "business.confirm",
        "payload": "[\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",{\"meta\":{\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"8\"},\"expected_head_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"kind\":\"production\",\"head_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T15:44:03.357Z\",\"updated_at\":\"2026-10-24T15:44:03.401Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"effective_confirmation_id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"previous_revision_id\":\"d3e4d800-3a9a-41e4-a61e-0dd9136af877\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"716cd1c4-a149-4669-8fc9-b24a06e83c48\",\"record_id\":\"ce3aa98c-0969-43fe-95ac-018ed35b076a\",\"revision_id\":\"d2d7b2b8-3e92-41e5-a5b1-801ed474451e\",\"request_id\":\"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4\",\"recorded_at\":\"2026-10-24T15:44:03.401Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"eca709b5-6102-4989-a707-06875ed6452c\"}},\"issues\":[]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"9\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "9"
      },
      {
        "request_id": "8aaa3c1f-380a-470a-9a61-644a09609ed5",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"8aaa3c1f-380a-470a-9a61-644a09609ed5\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.446Z\",\"updated_at\":\"2026-10-24T15:44:03.446Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"material_id\":\"2380fd77-6146-4527-930b-9146dd227347\",\"material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"ec23dc5d-e8fb-408c-89e0-901ebd21705d\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"1c4fe992-93f2-46e0-b931-54e13e2fe6cc\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.446Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"10\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "10"
      },
      {
        "request_id": "8281d185-dd20-43ae-a379-d265e1a0d8b1",
        "operation": "recipe.copy",
        "payload": "[\"2380fd77-6146-4527-930b-9146dd227347\",{\"meta\":{\"request_id\":\"8281d185-dd20-43ae-a379-d265e1a0d8b1\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"bf65c9c1-02c4-42c4-a09a-dabb4693df12\",\"source_recipe_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\"}]",
        "response": "{\"item\":{\"id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T15:44:03.507Z\",\"updated_at\":\"2026-10-24T15:44:03.507Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"de430f1d-d200-49d2-a105-d6eae2789fc1\",\"material_id\":\"310b632e-ded4-4006-b5b8-2ea6f8a48916\",\"material_version_id\":\"09a5de15-3d14-4d43-906a-8b4eb05d512d\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"71155bb2-a610-407f-ac3a-d3137b5b26a1\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"136240c0-08f2-42da-9428-a3c13d393efb\",\"position\":\"0\",\"material_id\":\"21edab83-b27b-4245-9192-af759cc3d5d6\",\"material_version_id\":\"ee8b1037-d509-49c8-a6df-1bdced14b5ea\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"9d7016fe-c82e-4782-9633-67154a20e5b7\",\"position\":\"1\",\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T15:44:03.507Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"11\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "11"
      },
      {
        "request_id": "0969c45f-7d37-4d86-8733-eae0b7609ea0",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"0969c45f-7d37-4d86-8733-eae0b7609ea0\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"9349d45d-6af3-4d0f-bf6c-536eed0c369a\",\"kind\":\"production\",\"head_id\":\"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.552Z\",\"updated_at\":\"2026-10-24T15:44:03.552Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":1,\"material_name\":null},\"head\":{\"id\":\"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9\",\"record_id\":\"9349d45d-6af3-4d0f-bf6c-536eed0c369a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"1597a96e-0e12-40c6-a696-481f491e49eb\",\"material_version_id\":\"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"recorded_at\":\"2026-10-24T15:44:03.552Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[{\"field\":\"outputs.0.material_id\",\"code\":\"missing_reference\",\"message\":\"생산의 모든 행에서 품목을 선택하세요\"}]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"12\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "12"
      },
      {
        "request_id": "a73d831d-975c-41f3-b93b-20ed1065db91",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"a73d831d-975c-41f3-b93b-20ed1065db91\",\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"expected_revision\":\"12\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입고\",\"stock_effect\":{\"kind\":\"needs_reconciliation\",\"basis_id\":null,\"included_quantity\":\"\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c5c92f47-3f0e-48cb-b250-1855131afa93\",\"kind\":\"receipt\",\"head_id\":\"2a08fc6a-3275-48aa-be8e-35a15f1a2f35\",\"status\":\"draft\",\"created_at\":\"2026-10-24T15:44:03.584Z\",\"updated_at\":\"2026-10-24T15:44:03.584Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":6,\"material_name\":null},\"head\":{\"id\":\"2a08fc6a-3275-48aa-be8e-35a15f1a2f35\",\"record_id\":\"c5c92f47-3f0e-48cb-b250-1855131afa93\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입고\",\"stock_effect\":{\"kind\":\"needs_reconciliation\",\"basis_id\":null,\"included_quantity\":\"\"}}},\"recorded_at\":\"2026-10-24T15:44:03.584Z\",\"seq\":\"13\"},\"confirmation\":null,\"issues\":[{\"field\":\"stock_effect\",\"code\":\"needs_reconciliation\",\"message\":\"이미 포함되었거나 확인이 필요한 입고는 초안으로 보관합니다. 보정 연결 전에는 확정할 수 없습니다\"},{\"field\":\"source.material_id\",\"code\":\"missing_reference\",\"message\":\"입고할 재료를 선택하세요\"},{\"field\":\"quantity\",\"code\":\"invalid_quantity\",\"message\":\"소수점 뒤의 숫자를 입력하세요\"},{\"field\":\"unit\",\"code\":\"invalid_unit\",\"message\":\"관리 단위는 제어문자 없이 1~20자로 입력하세요\"},{\"field\":\"amount\",\"code\":\"invalid_amount\",\"message\":\"0 이상의 십진수와 소수 최대 6자리로 입력하세요\"},{\"field\":\"receipt_date\",\"code\":\"invalid_date\",\"message\":\"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요\"}]},\"state\":{\"epoch\":\"5be978c7-7c35-499f-995b-8fade7437d89\",\"revision\":\"13\"}}",
        "epoch": "5be978c7-7c35-499f-995b-8fade7437d89",
        "sequence": "13"
      }
    ],
    "business_records": [
      {
        "id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "kind": "purchase_order",
        "created_at": "2026-10-24T15:44:03.126Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.234Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.357Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "kind": "production",
        "created_at": "2026-10-24T15:44:03.552Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      },
      {
        "id": "c5c92f47-3f0e-48cb-b250-1855131afa93",
        "kind": "receipt",
        "created_at": "2026-10-24T15:44:03.584Z",
        "created_sequence": "13",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.126Z",
        "seq": "3"
      },
      {
        "id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "previous_revision_id": "6e677cd5-0cb9-4e4c-a109-da922fa71994",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
            "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.234Z",
        "seq": "5"
      },
      {
        "id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "previous_revision_id": "bc9264f9-f153-4725-beda-a3b03223e464",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
              "expected_order_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
              "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"
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
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.357Z",
        "seq": "8"
      },
      {
        "id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "previous_revision_id": "d3e4d800-3a9a-41e4-a61e-0dd9136af877",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
                "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "1436d9e5-2239-43ba-b6e8-a8415e4d8fd9",
        "record_id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
                "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
                "quantity": "1.",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": null,
                "material_version_id": null,
                "quantity": "",
                "unit": "",
                "allocation_percent": ""
              }
            ],
            "note": "가상 미완성 생산"
          }
        },
        "recorded_at": "2026-10-24T15:44:03.552Z",
        "seq": "12"
      },
      {
        "id": "2a08fc6a-3275-48aa-be8e-35a15f1a2f35",
        "record_id": "c5c92f47-3f0e-48cb-b250-1855131afa93",
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
            "note": "가상 미완성 입고",
            "stock_effect": {
              "kind": "needs_reconciliation",
              "basis_id": null,
              "included_quantity": ""
            }
          }
        },
        "recorded_at": "2026-10-24T15:44:03.584Z",
        "seq": "13"
      }
    ],
    "business_heads": [
      {
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "current_revision_id": "5804b6c7-6275-4a59-8f04-d81935269804"
      },
      {
        "record_id": "9349d45d-6af3-4d0f-bf6c-536eed0c369a",
        "current_revision_id": "1436d9e5-2239-43ba-b6e8-a8415e4d8fd9"
      },
      {
        "record_id": "c5c92f47-3f0e-48cb-b250-1855131afa93",
        "current_revision_id": "2a08fc6a-3275-48aa-be8e-35a15f1a2f35"
      },
      {
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "current_revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e"
      },
      {
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "current_revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2"
      }
    ],
    "business_confirmations": [
      {
        "id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "revision_id": "5804b6c7-6275-4a59-8f04-d81935269804",
        "request_id": "b41f14d4-0178-42aa-ad48-0b56b02ea27b",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "revision_id": "121f07fd-8963-4592-b446-1bf66b7d53c2",
        "request_id": "15964307-a9f4-45cb-a3fd-768b20299bca",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "revision_id": "d2d7b2b8-3e92-41e5-a5b1-801ed474451e",
        "request_id": "f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "first_confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "created_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": null,
        "confirmation_id": "aa2ff928-ba6c-47f0-995c-7c08c71396ac",
        "business_record_id": "6de68baf-e40c-4022-9abe-3c2488e846cf",
        "action": "place",
        "status": "open",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.172Z",
        "seq": "4"
      },
      {
        "id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "previous_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "action": "receive",
        "status": "received",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "current_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad"
      }
    ],
    "receipts": [
      {
        "id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "business_record_id": "f1b82e50-afa2-42f1-9179-2c77eda8c8b2",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
          "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": null,
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.278Z",
        "seq": "6"
      },
      {
        "id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_production_effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "previous_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b",
        "receipt_id": null,
        "baseline_receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "production_effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "current_version_id": "55531488-21c3-4b3c-a341-2cda83ab4dd6",
        "last_valid_version_id": "73802c25-1db1-4ef9-ad91-8be379656b9b"
      },
      {
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "current_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52",
        "last_valid_version_id": "3fe35dd8-f103-4b71-b418-219a25659f52"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "0e6ba81f-cfa8-4315-b37a-6fcfaf416672",
        "receipt_id": "7b816f0b-3317-4fa4-8b00-1daecc37acef",
        "confirmation_id": "ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294",
        "initial_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "ordered_version_id": "482b5c06-a6e9-4fa7-8fe5-6230e7cf009a",
        "received_version_id": "b40aec9c-e468-44f3-a588-5905ac0b99ad",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "material_version_id": "bf65c9c1-02c4-42c4-a09a-dabb4693df12",
        "material_name": "가상 판매 메뉴",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": null,
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.446Z",
        "seq": "10"
      },
      {
        "id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "material_version_id": "09a5de15-3d14-4d43-906a-8b4eb05d512d",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T15:44:03.507Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "ec23dc5d-e8fb-408c-89e0-901ebd21705d",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1",
        "line": {
          "id": "1c4fe992-93f2-46e0-b931-54e13e2fe6cc",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "136240c0-08f2-42da-9428-a3c13d393efb",
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1",
        "line": {
          "id": "9d7016fe-c82e-4782-9633-67154a20e5b7",
          "position": "1",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
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
        "material_id": "2380fd77-6146-4527-930b-9146dd227347",
        "current_version_id": "71155bb2-a610-407f-ac3a-d3137b5b26a1"
      },
      {
        "material_id": "310b632e-ded4-4006-b5b8-2ea6f8a48916",
        "current_version_id": "de430f1d-d200-49d2-a105-d6eae2789fc1"
      }
    ],
    "productions": [
      {
        "id": "eca709b5-6102-4989-a707-06875ed6452c",
        "confirmation_id": "716cd1c4-a149-4669-8fc9-b24a06e83c48",
        "business_record_id": "ce3aa98c-0969-43fe-95ac-018ed35b076a",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "61c15365-446a-42e6-839e-6930a7667783",
        "row": {
          "position": "0",
          "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
          "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
          "material_name": "가상 판매 곡물",
          "quantity": "3000",
          "unit": "g",
          "input_quantity": "3.00",
          "input_unit": "kg",
          "value": "30000"
        }
      }
    ],
    "production_outputs": [
      {
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "effect_id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "row": {
          "position": "0",
          "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
          "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
          "material_name": "가상 판매 비축품",
          "quantity": "2000",
          "unit": "g",
          "input_quantity": "2",
          "input_unit": "kg",
          "value": "30000",
          "allocation_percent": "100"
        }
      }
    ],
    "production_effects": [
      {
        "id": "61c15365-446a-42e6-839e-6930a7667783",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "1597a96e-0e12-40c6-a696-481f491e49eb",
        "material_version_id": "1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      },
      {
        "id": "a0c2981a-0890-4a91-bc83-797f44de4342",
        "production_id": "eca709b5-6102-4989-a707-06875ed6452c",
        "material_id": "21edab83-b27b-4245-9192-af759cc3d5d6",
        "material_version_id": "ee8b1037-d509-49c8-a6df-1bdced14b5ea",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T15:44:03.401Z",
        "seq": "9"
      }
    ]
  }
}
',1);
INSERT INTO "material_versions" VALUES('1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7','1597a96e-0e12-40c6-a696-481f491e49eb','가상 판매 곡물','g','g','mass','raw','가상 이전 자료',0,'2026-10-24T15:44:03.064Z',1,'catalog');
INSERT INTO "material_versions" VALUES('ee8b1037-d509-49c8-a6df-1bdced14b5ea','21edab83-b27b-4245-9192-af759cc3d5d6','가상 판매 비축품','g','g','mass','prepared','가상 실제 결과',0,'2026-10-24T15:44:03.325Z',7,'catalog');
INSERT INTO "material_versions" VALUES('bf65c9c1-02c4-42c4-a09a-dabb4693df12','2380fd77-6146-4527-930b-9146dd227347','가상 판매 메뉴','개','개','custom','menu','가상 레시피',0,'2026-10-24T15:44:03.446Z',10,'catalog');
INSERT INTO "material_versions" VALUES('09a5de15-3d14-4d43-906a-8b4eb05d512d','310b632e-ded4-4006-b5b8-2ea6f8a48916','가상 판매 메뉴 (2)','개','개','custom','menu','가상 레시피',0,'2026-10-24T15:44:03.507Z',11,'catalog');
INSERT INTO "material_heads" VALUES('1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7');
INSERT INTO "material_heads" VALUES('21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea');
INSERT INTO "material_heads" VALUES('2380fd77-6146-4527-930b-9146dd227347','bf65c9c1-02c4-42c4-a09a-dabb4693df12');
INSERT INTO "material_heads" VALUES('310b632e-ded4-4006-b5b8-2ea6f8a48916','09a5de15-3d14-4d43-906a-8b4eb05d512d');
INSERT INTO "material_prices" VALUES('b4341c6d-3e62-4e71-853c-a4924153ffb5','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',1000000000,9000000000,'g','1','kg','2026-10-24T15:44:03.095Z',2);
INSERT INTO "mutation_requests" VALUES('837db76a-bf4c-4a33-a1cc-11b2064f4aaa','material.create','{"request_id":"837db76a-bf4c-4a33-a1cc-11b2064f4aaa","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"0","name":"가상 판매 곡물","unit":"g","kind":"raw","description":"가상 이전 자료"}','{"id":"1597a96e-0e12-40c6-a696-481f491e49eb","version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","name":"가상 판매 곡물","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"가상 이전 자료","archived":false,"created_at":"2026-10-24T15:44:03.064Z","updated_at":"2026-10-24T15:44:03.064Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"1"}','5be978c7-7c35-499f-995b-8fade7437d89',1);
INSERT INTO "mutation_requests" VALUES('5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6','material.price','["1597a96e-0e12-40c6-a696-481f491e49eb",{"request_id":"5cca6ac1-82d4-4fbc-b62e-6244a7cf6bc6","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"1","expected_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"1","amount":"9000","unit":"kg"}]','{"id":"b4341c6d-3e62-4e71-853c-a4924153ffb5","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"1000","amount":"9000","unit":"g","input_quantity":"1","input_unit":"kg","recorded_at":"2026-10-24T15:44:03.095Z","seq":"2","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"2"}','5be978c7-7c35-499f-995b-8fade7437d89',2);
INSERT INTO "mutation_requests" VALUES('e8378d48-03dc-45db-a38f-25477b854b80','business.create','{"meta":{"request_id":"e8378d48-03dc-45db-a38f-25477b854b80","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"2"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"6de68baf-e40c-4022-9abe-3c2488e846cf","kind":"purchase_order","head_id":"6e677cd5-0cb9-4e4c-a109-da922fa71994","status":"draft","created_at":"2026-10-24T15:44:03.126Z","updated_at":"2026-10-24T15:44:03.126Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"6e677cd5-0cb9-4e4c-a109-da922fa71994","record_id":"6de68baf-e40c-4022-9abe-3c2488e846cf","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T15:44:03.126Z","seq":"3"},"confirmation":null,"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"3"}}','5be978c7-7c35-499f-995b-8fade7437d89',3);
INSERT INTO "mutation_requests" VALUES('b41f14d4-0178-42aa-ad48-0b56b02ea27b','business.confirm','["6de68baf-e40c-4022-9abe-3c2488e846cf",{"meta":{"request_id":"b41f14d4-0178-42aa-ad48-0b56b02ea27b","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"3"},"expected_head_id":"6e677cd5-0cb9-4e4c-a109-da922fa71994"}]','{"result":{"item":{"id":"6de68baf-e40c-4022-9abe-3c2488e846cf","kind":"purchase_order","head_id":"5804b6c7-6275-4a59-8f04-d81935269804","status":"confirmed","created_at":"2026-10-24T15:44:03.126Z","updated_at":"2026-10-24T15:44:03.172Z","corrects_confirmation_id":null,"confirmation_id":"aa2ff928-ba6c-47f0-995c-7c08c71396ac","effective_confirmation_id":"aa2ff928-ba6c-47f0-995c-7c08c71396ac","issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"5804b6c7-6275-4a59-8f04-d81935269804","record_id":"6de68baf-e40c-4022-9abe-3c2488e846cf","previous_revision_id":"6e677cd5-0cb9-4e4c-a109-da922fa71994","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T15:44:03.172Z","seq":"4"},"confirmation":{"id":"aa2ff928-ba6c-47f0-995c-7c08c71396ac","record_id":"6de68baf-e40c-4022-9abe-3c2488e846cf","revision_id":"5804b6c7-6275-4a59-8f04-d81935269804","request_id":"b41f14d4-0178-42aa-ad48-0b56b02ea27b","recorded_at":"2026-10-24T15:44:03.172Z","seq":"4","result":{"kind":"purchase_order","action":"place","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"}},"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"4"}}','5be978c7-7c35-499f-995b-8fade7437d89',4);
INSERT INTO "mutation_requests" VALUES('314ef744-b77d-44b6-959e-96a06bc8aed3','business.create','{"meta":{"request_id":"314ef744-b77d-44b6-959e-96a06bc8aed3","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"4"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","expected_order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"f1b82e50-afa2-42f1-9179-2c77eda8c8b2","kind":"receipt","head_id":"bc9264f9-f153-4725-beda-a3b03223e464","status":"draft","created_at":"2026-10-24T15:44:03.234Z","updated_at":"2026-10-24T15:44:03.234Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"bc9264f9-f153-4725-beda-a3b03223e464","record_id":"f1b82e50-afa2-42f1-9179-2c77eda8c8b2","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","expected_order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T15:44:03.234Z","seq":"5"},"confirmation":null,"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"5"}}','5be978c7-7c35-499f-995b-8fade7437d89',5);
INSERT INTO "mutation_requests" VALUES('15964307-a9f4-45cb-a3fd-768b20299bca','business.confirm','["f1b82e50-afa2-42f1-9179-2c77eda8c8b2",{"meta":{"request_id":"15964307-a9f4-45cb-a3fd-768b20299bca","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"5"},"expected_head_id":"bc9264f9-f153-4725-beda-a3b03223e464"}]','{"result":{"item":{"id":"f1b82e50-afa2-42f1-9179-2c77eda8c8b2","kind":"receipt","head_id":"121f07fd-8963-4592-b446-1bf66b7d53c2","status":"confirmed","created_at":"2026-10-24T15:44:03.234Z","updated_at":"2026-10-24T15:44:03.278Z","corrects_confirmation_id":null,"confirmation_id":"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294","effective_confirmation_id":"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294","issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"121f07fd-8963-4592-b446-1bf66b7d53c2","record_id":"f1b82e50-afa2-42f1-9179-2c77eda8c8b2","previous_revision_id":"bc9264f9-f153-4725-beda-a3b03223e464","status":"confirmed","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","expected_order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T15:44:03.278Z","seq":"6"},"confirmation":{"id":"ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294","record_id":"f1b82e50-afa2-42f1-9179-2c77eda8c8b2","revision_id":"121f07fd-8963-4592-b446-1bf66b7d53c2","request_id":"15964307-a9f4-45cb-a3fd-768b20299bca","recorded_at":"2026-10-24T15:44:03.278Z","seq":"6","result":{"kind":"receipt","receipt_id":"7b816f0b-3317-4fa4-8b00-1daecc37acef","holding_version_id":"73802c25-1db1-4ef9-ad91-8be379656b9b","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","order_version_id":"b40aec9c-e468-44f3-a588-5905ac0b99ad"}},"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"6"}}','5be978c7-7c35-499f-995b-8fade7437d89',6);
INSERT INTO "mutation_requests" VALUES('f159c8bb-07ca-4551-94f2-a4a8015d98be','material.create','{"request_id":"f159c8bb-07ca-4551-94f2-a4a8015d98be","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"6","name":"가상 판매 비축품","unit":"g","kind":"prepared","description":"가상 실제 결과"}','{"id":"21edab83-b27b-4245-9192-af759cc3d5d6","version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","name":"가상 판매 비축품","unit":"g","canonical_unit":"g","dimension":"mass","kind":"prepared","description":"가상 실제 결과","archived":false,"created_at":"2026-10-24T15:44:03.325Z","updated_at":"2026-10-24T15:44:03.325Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"7"}','5be978c7-7c35-499f-995b-8fade7437d89',7);
INSERT INTO "mutation_requests" VALUES('9f429bd6-c535-4103-932d-e2ce4878bf0a','business.create','{"meta":{"request_id":"9f429bd6-c535-4103-932d-e2ce4878bf0a","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"7"},"payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"ce3aa98c-0969-43fe-95ac-018ed35b076a","kind":"production","head_id":"d3e4d800-3a9a-41e4-a61e-0dd9136af877","status":"draft","created_at":"2026-10-24T15:44:03.357Z","updated_at":"2026-10-24T15:44:03.357Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 비축품"},"head":{"id":"d3e4d800-3a9a-41e4-a61e-0dd9136af877","record_id":"ce3aa98c-0969-43fe-95ac-018ed35b076a","previous_revision_id":null,"status":"draft","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"recorded_at":"2026-10-24T15:44:03.357Z","seq":"8"},"confirmation":null,"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"8"}}','5be978c7-7c35-499f-995b-8fade7437d89',8);
INSERT INTO "mutation_requests" VALUES('f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4','business.confirm','["ce3aa98c-0969-43fe-95ac-018ed35b076a",{"meta":{"request_id":"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"8"},"expected_head_id":"d3e4d800-3a9a-41e4-a61e-0dd9136af877"}]','{"result":{"item":{"id":"ce3aa98c-0969-43fe-95ac-018ed35b076a","kind":"production","head_id":"d2d7b2b8-3e92-41e5-a5b1-801ed474451e","status":"confirmed","created_at":"2026-10-24T15:44:03.357Z","updated_at":"2026-10-24T15:44:03.401Z","corrects_confirmation_id":null,"confirmation_id":"716cd1c4-a149-4669-8fc9-b24a06e83c48","effective_confirmation_id":"716cd1c4-a149-4669-8fc9-b24a06e83c48","issue_count":0,"material_name":"가상 판매 비축품"},"head":{"id":"d2d7b2b8-3e92-41e5-a5b1-801ed474451e","record_id":"ce3aa98c-0969-43fe-95ac-018ed35b076a","previous_revision_id":"d3e4d800-3a9a-41e4-a61e-0dd9136af877","status":"confirmed","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"recorded_at":"2026-10-24T15:44:03.401Z","seq":"9"},"confirmation":{"id":"716cd1c4-a149-4669-8fc9-b24a06e83c48","record_id":"ce3aa98c-0969-43fe-95ac-018ed35b076a","revision_id":"d2d7b2b8-3e92-41e5-a5b1-801ed474451e","request_id":"f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4","recorded_at":"2026-10-24T15:44:03.401Z","seq":"9","result":{"kind":"production","production_id":"eca709b5-6102-4989-a707-06875ed6452c"}},"issues":[]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"9"}}','5be978c7-7c35-499f-995b-8fade7437d89',9);
INSERT INTO "mutation_requests" VALUES('8aaa3c1f-380a-470a-9a61-644a09609ed5','recipe.create','{"meta":{"request_id":"8aaa3c1f-380a-470a-9a61-644a09609ed5","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"9"},"owner":{"mode":"new","name":"가상 판매 메뉴","unit":"개","kind":"menu","description":"가상 레시피"},"data":{"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"100.00","unit":"g","note":""},{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"0","unit":"g","note":"가상 적당량"}]}}','{"item":{"id":"2380fd77-6146-4527-930b-9146dd227347","version_id":"bf65c9c1-02c4-42c4-a09a-dabb4693df12","name":"가상 판매 메뉴","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T15:44:03.446Z","updated_at":"2026-10-24T15:44:03.446Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"71155bb2-a610-407f-ac3a-d3137b5b26a1","material_id":"2380fd77-6146-4527-930b-9146dd227347","material_version_id":"bf65c9c1-02c4-42c4-a09a-dabb4693df12","material_name":"가상 판매 메뉴","material_unit":"개","previous_version_id":null,"copied_from_version_id":null,"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"ec23dc5d-e8fb-408c-89e0-901ebd21705d","position":"0","material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","material_name":"가상 판매 비축품","quantity":"100","unit":"g","input_quantity":"100.00","input_unit":"g","note":""},{"id":"1c4fe992-93f2-46e0-b931-54e13e2fe6cc","position":"1","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","material_name":"가상 판매 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T15:44:03.446Z","seq":"10"},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"10"}}','5be978c7-7c35-499f-995b-8fade7437d89',10);
INSERT INTO "mutation_requests" VALUES('8281d185-dd20-43ae-a379-d265e1a0d8b1','recipe.copy','["2380fd77-6146-4527-930b-9146dd227347",{"meta":{"request_id":"8281d185-dd20-43ae-a379-d265e1a0d8b1","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"10"},"expected_material_version_id":"bf65c9c1-02c4-42c4-a09a-dabb4693df12","source_recipe_version_id":"71155bb2-a610-407f-ac3a-d3137b5b26a1"}]','{"item":{"id":"310b632e-ded4-4006-b5b8-2ea6f8a48916","version_id":"09a5de15-3d14-4d43-906a-8b4eb05d512d","name":"가상 판매 메뉴 (2)","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T15:44:03.507Z","updated_at":"2026-10-24T15:44:03.507Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"de430f1d-d200-49d2-a105-d6eae2789fc1","material_id":"310b632e-ded4-4006-b5b8-2ea6f8a48916","material_version_id":"09a5de15-3d14-4d43-906a-8b4eb05d512d","material_name":"가상 판매 메뉴 (2)","material_unit":"개","previous_version_id":null,"copied_from_version_id":"71155bb2-a610-407f-ac3a-d3137b5b26a1","basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"136240c0-08f2-42da-9428-a3c13d393efb","position":"0","material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","material_name":"가상 판매 비축품","quantity":"100","unit":"g","input_quantity":"100.00","input_unit":"g","note":""},{"id":"9d7016fe-c82e-4782-9633-67154a20e5b7","position":"1","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","material_name":"가상 판매 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T15:44:03.507Z","seq":"11"},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"11"}}','5be978c7-7c35-499f-995b-8fade7437d89',11);
INSERT INTO "mutation_requests" VALUES('0969c45f-7d37-4d86-8733-eae0b7609ea0','business.create','{"meta":{"request_id":"0969c45f-7d37-4d86-8733-eae0b7609ea0","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"11"},"payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"9349d45d-6af3-4d0f-bf6c-536eed0c369a","kind":"production","head_id":"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9","status":"draft","created_at":"2026-10-24T15:44:03.552Z","updated_at":"2026-10-24T15:44:03.552Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":1,"material_name":null},"head":{"id":"1436d9e5-2239-43ba-b6e8-a8415e4d8fd9","record_id":"9349d45d-6af3-4d0f-bf6c-536eed0c369a","previous_revision_id":null,"status":"draft","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}},"recorded_at":"2026-10-24T15:44:03.552Z","seq":"12"},"confirmation":null,"issues":[{"field":"outputs.0.material_id","code":"missing_reference","message":"생산의 모든 행에서 품목을 선택하세요"}]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"12"}}','5be978c7-7c35-499f-995b-8fade7437d89',12);
INSERT INTO "mutation_requests" VALUES('a73d831d-975c-41f3-b93b-20ed1065db91','business.create','{"meta":{"request_id":"a73d831d-975c-41f3-b93b-20ed1065db91","epoch":"5be978c7-7c35-499f-995b-8fade7437d89","expected_revision":"12"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"c5c92f47-3f0e-48cb-b250-1855131afa93","kind":"receipt","head_id":"2a08fc6a-3275-48aa-be8e-35a15f1a2f35","status":"draft","created_at":"2026-10-24T15:44:03.584Z","updated_at":"2026-10-24T15:44:03.584Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":6,"material_name":null},"head":{"id":"2a08fc6a-3275-48aa-be8e-35a15f1a2f35","record_id":"c5c92f47-3f0e-48cb-b250-1855131afa93","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}},"recorded_at":"2026-10-24T15:44:03.584Z","seq":"13"},"confirmation":null,"issues":[{"field":"stock_effect","code":"needs_reconciliation","message":"이미 포함되었거나 확인이 필요한 입고는 초안으로 보관합니다. 보정 연결 전에는 확정할 수 없습니다"},{"field":"source.material_id","code":"missing_reference","message":"입고할 재료를 선택하세요"},{"field":"quantity","code":"invalid_quantity","message":"소수점 뒤의 숫자를 입력하세요"},{"field":"unit","code":"invalid_unit","message":"관리 단위는 제어문자 없이 1~20자로 입력하세요"},{"field":"amount","code":"invalid_amount","message":"0 이상의 십진수와 소수 최대 6자리로 입력하세요"},{"field":"receipt_date","code":"invalid_date","message":"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요"}]},"state":{"epoch":"5be978c7-7c35-499f-995b-8fade7437d89","revision":"13"}}','5be978c7-7c35-499f-995b-8fade7437d89',13);
INSERT INTO "catalog_state" VALUES(1,'5be978c7-7c35-499f-995b-8fade7437d89',13);
INSERT INTO "business_records" VALUES('6de68baf-e40c-4022-9abe-3c2488e846cf','purchase_order','2026-10-24T15:44:03.126Z',3,NULL);
INSERT INTO "business_records" VALUES('f1b82e50-afa2-42f1-9179-2c77eda8c8b2','receipt','2026-10-24T15:44:03.234Z',5,NULL);
INSERT INTO "business_records" VALUES('ce3aa98c-0969-43fe-95ac-018ed35b076a','production','2026-10-24T15:44:03.357Z',8,NULL);
INSERT INTO "business_records" VALUES('9349d45d-6af3-4d0f-bf6c-536eed0c369a','production','2026-10-24T15:44:03.552Z',12,NULL);
INSERT INTO "business_records" VALUES('c5c92f47-3f0e-48cb-b250-1855131afa93','receipt','2026-10-24T15:44:03.584Z',13,NULL);
INSERT INTO "business_revisions" VALUES('6e677cd5-0cb9-4e4c-a109-da922fa71994','6de68baf-e40c-4022-9abe-3c2488e846cf',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T15:44:03.126Z',3);
INSERT INTO "business_revisions" VALUES('5804b6c7-6275-4a59-8f04-d81935269804','6de68baf-e40c-4022-9abe-3c2488e846cf','6e677cd5-0cb9-4e4c-a109-da922fa71994','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T15:44:03.172Z',4);
INSERT INTO "business_revisions" VALUES('bc9264f9-f153-4725-beda-a3b03223e464','f1b82e50-afa2-42f1-9179-2c77eda8c8b2',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","expected_order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T15:44:03.234Z',5);
INSERT INTO "business_revisions" VALUES('121f07fd-8963-4592-b446-1bf66b7d53c2','f1b82e50-afa2-42f1-9179-2c77eda8c8b2','bc9264f9-f153-4725-beda-a3b03223e464','confirmed',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"0e6ba81f-cfa8-4315-b37a-6fcfaf416672","expected_order_version_id":"482b5c06-a6e9-4fa7-8fe5-6230e7cf009a","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T15:44:03.278Z',6);
INSERT INTO "business_revisions" VALUES('d3e4d800-3a9a-41e4-a61e-0dd9136af877','ce3aa98c-0969-43fe-95ac-018ed35b076a',NULL,'draft',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}}','2026-10-24T15:44:03.357Z',8);
INSERT INTO "business_revisions" VALUES('d2d7b2b8-3e92-41e5-a5b1-801ed474451e','ce3aa98c-0969-43fe-95ac-018ed35b076a','d3e4d800-3a9a-41e4-a61e-0dd9136af877','confirmed',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"21edab83-b27b-4245-9192-af759cc3d5d6","material_version_id":"ee8b1037-d509-49c8-a6df-1bdced14b5ea","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}}','2026-10-24T15:44:03.401Z',9);
INSERT INTO "business_revisions" VALUES('1436d9e5-2239-43ba-b6e8-a8415e4d8fd9','9349d45d-6af3-4d0f-bf6c-536eed0c369a',NULL,'draft',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"1597a96e-0e12-40c6-a696-481f491e49eb","material_version_id":"1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}}','2026-10-24T15:44:03.552Z',12);
INSERT INTO "business_revisions" VALUES('2a08fc6a-3275-48aa-be8e-35a15f1a2f35','c5c92f47-3f0e-48cb-b250-1855131afa93',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}}','2026-10-24T15:44:03.584Z',13);
INSERT INTO "business_heads" VALUES('6de68baf-e40c-4022-9abe-3c2488e846cf','5804b6c7-6275-4a59-8f04-d81935269804');
INSERT INTO "business_heads" VALUES('f1b82e50-afa2-42f1-9179-2c77eda8c8b2','121f07fd-8963-4592-b446-1bf66b7d53c2');
INSERT INTO "business_heads" VALUES('ce3aa98c-0969-43fe-95ac-018ed35b076a','d2d7b2b8-3e92-41e5-a5b1-801ed474451e');
INSERT INTO "business_heads" VALUES('9349d45d-6af3-4d0f-bf6c-536eed0c369a','1436d9e5-2239-43ba-b6e8-a8415e4d8fd9');
INSERT INTO "business_heads" VALUES('c5c92f47-3f0e-48cb-b250-1855131afa93','2a08fc6a-3275-48aa-be8e-35a15f1a2f35');
INSERT INTO "business_confirmations" VALUES('aa2ff928-ba6c-47f0-995c-7c08c71396ac','6de68baf-e40c-4022-9abe-3c2488e846cf','5804b6c7-6275-4a59-8f04-d81935269804','b41f14d4-0178-42aa-ad48-0b56b02ea27b','2026-10-24T15:44:03.172Z',4);
INSERT INTO "business_confirmations" VALUES('ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294','f1b82e50-afa2-42f1-9179-2c77eda8c8b2','121f07fd-8963-4592-b446-1bf66b7d53c2','15964307-a9f4-45cb-a3fd-768b20299bca','2026-10-24T15:44:03.278Z',6);
INSERT INTO "business_confirmations" VALUES('716cd1c4-a149-4669-8fc9-b24a06e83c48','ce3aa98c-0969-43fe-95ac-018ed35b076a','d2d7b2b8-3e92-41e5-a5b1-801ed474451e','f2f5c4e7-b76e-44de-99b5-5ad2bd8e8df4','2026-10-24T15:44:03.401Z',9);
INSERT INTO "purchase_orders" VALUES('0e6ba81f-cfa8-4315-b37a-6fcfaf416672','aa2ff928-ba6c-47f0-995c-7c08c71396ac','2026-10-24T15:44:03.172Z',4);
INSERT INTO "purchase_order_versions" VALUES('482b5c06-a6e9-4fa7-8fe5-6230e7cf009a','0e6ba81f-cfa8-4315-b37a-6fcfaf416672',NULL,'aa2ff928-ba6c-47f0-995c-7c08c71396ac','place','open','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T15:44:03.172Z',4);
INSERT INTO "purchase_order_versions" VALUES('b40aec9c-e468-44f3-a588-5905ac0b99ad','0e6ba81f-cfa8-4315-b37a-6fcfaf416672','482b5c06-a6e9-4fa7-8fe5-6230e7cf009a','ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294','receive','received','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T15:44:03.278Z',6);
INSERT INTO "purchase_order_heads" VALUES('0e6ba81f-cfa8-4315-b37a-6fcfaf416672','b40aec9c-e468-44f3-a588-5905ac0b99ad');
INSERT INTO "receipts" VALUES('7b816f0b-3317-4fa4-8b00-1daecc37acef','ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',3000000000,'g','3','kg',30000000000,'2026-10-01','가상 실제 입고','2026-10-24T15:44:03.278Z',6,'order','0e6ba81f-cfa8-4315-b37a-6fcfaf416672','482b5c06-a6e9-4fa7-8fe5-6230e7cf009a');
INSERT INTO "order_receipt_closures" VALUES('0e6ba81f-cfa8-4315-b37a-6fcfaf416672','7b816f0b-3317-4fa4-8b00-1daecc37acef','ed3209d2-6fe5-4dd3-bf80-bdf4dee8d294','482b5c06-a6e9-4fa7-8fe5-6230e7cf009a','482b5c06-a6e9-4fa7-8fe5-6230e7cf009a','b40aec9c-e468-44f3-a588-5905ac0b99ad','3000','0');
INSERT INTO "recipe_versions" VALUES('71155bb2-a610-407f-ac3a-d3137b5b26a1','2380fd77-6146-4527-930b-9146dd227347','bf65c9c1-02c4-42c4-a09a-dabb4693df12',NULL,NULL,3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T15:44:03.446Z',10);
INSERT INTO "recipe_versions" VALUES('de430f1d-d200-49d2-a105-d6eae2789fc1','310b632e-ded4-4006-b5b8-2ea6f8a48916','09a5de15-3d14-4d43-906a-8b4eb05d512d',NULL,'71155bb2-a610-407f-ac3a-d3137b5b26a1',3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T15:44:03.507Z',11);
INSERT INTO "recipe_lines" VALUES('ec23dc5d-e8fb-408c-89e0-901ebd21705d','71155bb2-a610-407f-ac3a-d3137b5b26a1',0,'21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea',100000000,'g','100.00','g','');
INSERT INTO "recipe_lines" VALUES('1c4fe992-93f2-46e0-b931-54e13e2fe6cc','71155bb2-a610-407f-ac3a-d3137b5b26a1',1,'1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_lines" VALUES('136240c0-08f2-42da-9428-a3c13d393efb','de430f1d-d200-49d2-a105-d6eae2789fc1',0,'21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea',100000000,'g','100.00','g','');
INSERT INTO "recipe_lines" VALUES('9d7016fe-c82e-4782-9633-67154a20e5b7','de430f1d-d200-49d2-a105-d6eae2789fc1',1,'1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_heads" VALUES('2380fd77-6146-4527-930b-9146dd227347','71155bb2-a610-407f-ac3a-d3137b5b26a1');
INSERT INTO "recipe_heads" VALUES('310b632e-ded4-4006-b5b8-2ea6f8a48916','de430f1d-d200-49d2-a105-d6eae2789fc1');
INSERT INTO "productions" VALUES('eca709b5-6102-4989-a707-06875ed6452c','716cd1c4-a149-4669-8fc9-b24a06e83c48','가상 전량 생산',30000000000,'2026-10-24T15:44:03.401Z',9);
INSERT INTO "production_effects" VALUES('61c15365-446a-42e6-839e-6930a7667783','eca709b5-6102-4989-a707-06875ed6452c','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',3000000000,30000000000,0,0);
INSERT INTO "production_effects" VALUES('a0c2981a-0890-4a91-bc83-797f44de4342','eca709b5-6102-4989-a707-06875ed6452c','21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea',0,0,2000000000,30000000000);
INSERT INTO "production_inputs" VALUES('eca709b5-6102-4989-a707-06875ed6452c',0,'61c15365-446a-42e6-839e-6930a7667783','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',3000000000,'g','3.00','kg',30000000000);
INSERT INTO "production_outputs" VALUES('eca709b5-6102-4989-a707-06875ed6452c',0,'a0c2981a-0890-4a91-bc83-797f44de4342','21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea',2000000000,'g','2','kg',30000000000,100000000);
INSERT INTO "holding_versions" VALUES('73802c25-1db1-4ef9-ad91-8be379656b9b','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7',NULL,'7b816f0b-3317-4fa4-8b00-1daecc37acef','7b816f0b-3317-4fa4-8b00-1daecc37acef',6,3000000000,30000000000,'2026-10-24T15:44:03.278Z',6,NULL,NULL);
INSERT INTO "holding_versions" VALUES('55531488-21c3-4b3c-a341-2cda83ab4dd6','1597a96e-0e12-40c6-a696-481f491e49eb','1b76cb3f-b2b3-4050-ba18-d2ba76d1fff7','73802c25-1db1-4ef9-ad91-8be379656b9b',NULL,'7b816f0b-3317-4fa4-8b00-1daecc37acef',6,0,0,'2026-10-24T15:44:03.401Z',9,'61c15365-446a-42e6-839e-6930a7667783',NULL);
INSERT INTO "holding_versions" VALUES('3fe35dd8-f103-4b71-b418-219a25659f52','21edab83-b27b-4245-9192-af759cc3d5d6','ee8b1037-d509-49c8-a6df-1bdced14b5ea',NULL,NULL,NULL,9,2000000000,30000000000,'2026-10-24T15:44:03.401Z',9,'a0c2981a-0890-4a91-bc83-797f44de4342','a0c2981a-0890-4a91-bc83-797f44de4342');
INSERT INTO "holding_heads" VALUES('1597a96e-0e12-40c6-a696-481f491e49eb','55531488-21c3-4b3c-a341-2cda83ab4dd6','73802c25-1db1-4ef9-ad91-8be379656b9b');
INSERT INTO "holding_heads" VALUES('21edab83-b27b-4245-9192-af759cc3d5d6','3fe35dd8-f103-4b71-b418-219a25659f52','3fe35dd8-f103-4b71-b418-219a25659f52');
PRAGMA user_version=8;
