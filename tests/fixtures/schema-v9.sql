-- Frozen synthetic T-009 schema9 state captured through its release API.
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
CREATE TABLE sales(
        id TEXT PRIMARY KEY NOT NULL,original_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
        material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,recipe_version_id TEXT NOT NULL,
        basis_count INTEGER NOT NULL CHECK(basis_count>0 AND basis_count<=1000000000),
        selling_price_micros INTEGER CHECK(selling_price_micros>=0 AND selling_price_micros<=9000000000000000000),
        known_subtotal_micros INTEGER NOT NULL CHECK(known_subtotal_micros>=0 AND known_subtotal_micros<=9000000000000000000),
        total_cost_micros INTEGER CHECK(total_cost_micros>=0 AND total_cost_micros<=9000000000000000000),
        ratio_json TEXT NOT NULL,ratio_reason_json TEXT NOT NULL,
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
        FOREIGN KEY(material_id,recipe_version_id) REFERENCES recipe_versions(material_id,id));
CREATE TABLE sale_cost_rows(
        sale_id TEXT NOT NULL REFERENCES sales(id),position INTEGER NOT NULL CHECK(position>=0 AND position<100),
        recipe_line_id TEXT NOT NULL REFERENCES recipe_lines(id),source_kind TEXT NOT NULL CHECK(source_kind IN('holding_average','last_valid','preregistered','missing')),
        holding_version_id TEXT REFERENCES holding_versions(id),material_price_id TEXT REFERENCES material_prices(id),
        price_amount_micros INTEGER CHECK(price_amount_micros>=0 AND price_amount_micros<=9000000000000000000),
        price_quantity_micros INTEGER CHECK(price_quantity_micros>0 AND price_quantity_micros<=9000000000000000),
        missing_reason TEXT,cost_micros INTEGER CHECK(cost_micros>=0 AND cost_micros<=9000000000000000000),
        PRIMARY KEY(sale_id,position),UNIQUE(sale_id,recipe_line_id),
        CHECK((source_kind='missing' AND holding_version_id IS NULL AND material_price_id IS NULL AND price_amount_micros IS NULL AND price_quantity_micros IS NULL AND missing_reason IS NOT NULL)
           OR (source_kind='preregistered' AND holding_version_id IS NULL AND material_price_id IS NOT NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)
           OR (source_kind IN('holding_average','last_valid') AND holding_version_id IS NOT NULL AND material_price_id IS NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)));
CREATE TABLE sale_versions(
        id TEXT PRIMARY KEY NOT NULL,sale_id TEXT NOT NULL REFERENCES sales(id),previous_version_id TEXT,
        confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),count INTEGER NOT NULL CHECK(count>=0 AND count<=1000000000),
        amount_micros INTEGER CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),note TEXT NOT NULL,
        UNIQUE(sale_id,id),FOREIGN KEY(sale_id,previous_version_id) REFERENCES sale_versions(sale_id,id));
CREATE TABLE sale_heads(
        sale_id TEXT PRIMARY KEY NOT NULL REFERENCES sales(id),current_version_id TEXT NOT NULL,
        FOREIGN KEY(sale_id,current_version_id) REFERENCES sale_versions(sale_id,id));
CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
CREATE INDEX receipts_material ON receipts(material_id,sequence);
CREATE INDEX recipe_versions_material ON recipe_versions(material_id,sequence);
CREATE INDEX recipe_lines_material ON recipe_lines(material_id,recipe_version_id);
CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
CREATE INDEX production_effects_material ON production_effects(material_id,production_id);
CREATE INDEX sales_material ON sales(material_id);
CREATE INDEX sale_versions_sale ON sale_versions(sale_id);
INSERT INTO "materials" VALUES('5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','가상 판매 곡물','g','2026-10-24T16:28:41.516Z');
INSERT INTO "materials" VALUES('2167401e-86fc-4e3a-b7d1-dbde01bf41e1','가상 판매 비축품','g','2026-10-24T16:28:41.761Z');
INSERT INTO "materials" VALUES('3c339cdf-f952-4399-bc70-a9541335a732','가상 판매 메뉴','개','2026-10-24T16:28:41.887Z');
INSERT INTO "materials" VALUES('8c6d323c-0ff3-4b48-a7bb-ebca8616dd95','가상 판매 메뉴 (2)','개','2026-10-24T16:28:41.931Z');
INSERT INTO "snapshot_outbox" VALUES(1,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "1",
  "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
  "recorded_at": "2026-10-24T16:28:41.516Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "1"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(2,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "2",
  "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
  "recorded_at": "2026-10-24T16:28:41.547Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "2"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(3,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "3",
  "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
  "recorded_at": "2026-10-24T16:28:41.580Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "3"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2"
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(4,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "4",
  "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
  "recorded_at": "2026-10-24T16:28:41.608Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "4"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(5,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "5",
  "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
  "recorded_at": "2026-10-24T16:28:41.669Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "5"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(6,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "6",
  "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
  "recorded_at": "2026-10-24T16:28:41.715Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "6"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(7,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "7",
  "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
  "recorded_at": "2026-10-24T16:28:41.761Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "7"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(8,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "8",
  "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
  "recorded_at": "2026-10-24T16:28:41.807Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "8"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "holding_heads": [
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
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
    "production_effects": [],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(9,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "9",
  "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
  "recorded_at": "2026-10-24T16:28:41.853Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "9"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [],
    "recipe_lines": [],
    "recipe_heads": [],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(10,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "10",
  "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
  "recorded_at": "2026-10-24T16:28:41.887Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "10"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(11,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "11",
  "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
  "recorded_at": "2026-10-24T16:28:41.931Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "11"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(12,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "12",
  "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
  "recorded_at": "2026-10-24T16:28:42.009Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "12"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [],
    "sale_versions": [],
    "sale_heads": [],
    "sale_cost_rows": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(13,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "13",
  "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
  "recorded_at": "2026-10-24T16:28:42.054Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "13"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      },
      {
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "operation": "business.confirm",
        "payload": "[\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",{\"meta\":{\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"12\"},\"expected_head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.054Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\"},\"confirmation\":{\"id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"revision_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"61a30394-a9d3-4116-bd7e-b099d5f539ed\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"13\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "13"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      },
      {
        "id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [
      {
        "id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "original_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_name": "가상 판매 메뉴",
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "selling_price": "1000",
        "known_subtotal": "500",
        "total_cost": "500",
        "cost_ratio": {
          "numerator": "500000000",
          "denominator": "1000000000"
        },
        "ratio_unavailable_reason": null,
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_versions": [
      {
        "id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": null,
        "confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "count": "3",
        "amount": "3000",
        "note": "가상 원판매",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_heads": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "current_version_id": "61a30394-a9d3-4116-bd7e-b099d5f539ed"
      }
    ],
    "sale_cost_rows": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "0",
        "row": {
          "line_id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "numerator_quantity": "100",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "holding_average",
            "source_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
            "amount": "30000",
            "quantity": "2000",
            "reason": null
          },
          "cost": "500"
        }
      },
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "1",
        "row": {
          "line_id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "numerator_quantity": "0",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "last_valid",
            "source_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
            "amount": "30000",
            "quantity": "3000",
            "reason": null
          },
          "cost": "0"
        }
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(14,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "14",
  "request_id": "2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6",
  "recorded_at": "2026-10-24T16:28:42.134Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "14"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      },
      {
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "operation": "business.confirm",
        "payload": "[\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",{\"meta\":{\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"12\"},\"expected_head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.054Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\"},\"confirmation\":{\"id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"revision_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"61a30394-a9d3-4116-bd7e-b099d5f539ed\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"13\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "13"
      },
      {
        "request_id": "2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"13\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.134Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":null,\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.134Z\",\"seq\":\"14\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"14\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "14"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      },
      {
        "id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.134Z",
        "created_sequence": "14",
        "corrects_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf"
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      },
      {
        "id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.134Z",
        "seq": "14"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "current_revision_id": "df145c74-6e5a-4372-b633-cab024a4228e"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [
      {
        "id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "original_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_name": "가상 판매 메뉴",
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "selling_price": "1000",
        "known_subtotal": "500",
        "total_cost": "500",
        "cost_ratio": {
          "numerator": "500000000",
          "denominator": "1000000000"
        },
        "ratio_unavailable_reason": null,
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_versions": [
      {
        "id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": null,
        "confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "count": "3",
        "amount": "3000",
        "note": "가상 원판매",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_heads": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "current_version_id": "61a30394-a9d3-4116-bd7e-b099d5f539ed"
      }
    ],
    "sale_cost_rows": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "0",
        "row": {
          "line_id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "numerator_quantity": "100",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "holding_average",
            "source_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
            "amount": "30000",
            "quantity": "2000",
            "reason": null
          },
          "cost": "500"
        }
      },
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "1",
        "row": {
          "line_id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "numerator_quantity": "0",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "last_valid",
            "source_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
            "amount": "30000",
            "quantity": "3000",
            "reason": null
          },
          "cost": "0"
        }
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(15,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "15",
  "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
  "recorded_at": "2026-10-24T16:28:42.192Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "15"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      },
      {
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "operation": "business.confirm",
        "payload": "[\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",{\"meta\":{\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"12\"},\"expected_head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.054Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\"},\"confirmation\":{\"id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"revision_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"61a30394-a9d3-4116-bd7e-b099d5f539ed\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"13\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "13"
      },
      {
        "request_id": "2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"13\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.134Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":null,\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.134Z\",\"seq\":\"14\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"14\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "14"
      },
      {
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "operation": "business.confirm",
        "payload": "[\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",{\"meta\":{\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"14\"},\"expected_head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.192Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"effective_confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\"},\"confirmation\":{\"id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"revision_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"109b3cf2-0176-41dd-ba5d-a6d165b6551a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"15\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "15"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      },
      {
        "id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.134Z",
        "created_sequence": "14",
        "corrects_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf"
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      },
      {
        "id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.134Z",
        "seq": "14"
      },
      {
        "id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "current_revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [
      {
        "target_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "replacement_confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e"
      }
    ],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [
      {
        "id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "original_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_name": "가상 판매 메뉴",
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "selling_price": "1000",
        "known_subtotal": "500",
        "total_cost": "500",
        "cost_ratio": {
          "numerator": "500000000",
          "denominator": "1000000000"
        },
        "ratio_unavailable_reason": null,
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_versions": [
      {
        "id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": null,
        "confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "count": "3",
        "amount": "3000",
        "note": "가상 원판매",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "business_record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "count": "1",
        "amount": "1000",
        "note": "가상 판매 개수 정정",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "sale_heads": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "current_version_id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a"
      }
    ],
    "sale_cost_rows": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "0",
        "row": {
          "line_id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "numerator_quantity": "100",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "holding_average",
            "source_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
            "amount": "30000",
            "quantity": "2000",
            "reason": null
          },
          "cost": "500"
        }
      },
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "1",
        "row": {
          "line_id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "numerator_quantity": "0",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "last_valid",
            "source_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
            "amount": "30000",
            "quantity": "3000",
            "reason": null
          },
          "cost": "0"
        }
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(16,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "16",
  "request_id": "dbd9c969-dd60-4b6b-9322-01b591afb239",
  "recorded_at": "2026-10-24T16:28:42.286Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "16"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      },
      {
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "operation": "business.confirm",
        "payload": "[\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",{\"meta\":{\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"12\"},\"expected_head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.054Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\"},\"confirmation\":{\"id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"revision_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"61a30394-a9d3-4116-bd7e-b099d5f539ed\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"13\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "13"
      },
      {
        "request_id": "2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"13\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.134Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":null,\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.134Z\",\"seq\":\"14\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"14\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "14"
      },
      {
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "operation": "business.confirm",
        "payload": "[\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",{\"meta\":{\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"14\"},\"expected_head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.192Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"effective_confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\"},\"confirmation\":{\"id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"revision_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"109b3cf2-0176-41dd-ba5d-a6d165b6551a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"15\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "15"
      },
      {
        "request_id": "dbd9c969-dd60-4b6b-9322-01b591afb239",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"dbd9c969-dd60-4b6b-9322-01b591afb239\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"15\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"d0746729-8f85-494f-8f0e-22c8137563b5\",\"kind\":\"production\",\"head_id\":\"d3e859d5-5fb3-40a8-969a-f5b8456786cc\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.286Z\",\"updated_at\":\"2026-10-24T16:28:42.286Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":1,\"material_name\":null},\"head\":{\"id\":\"d3e859d5-5fb3-40a8-969a-f5b8456786cc\",\"record_id\":\"d0746729-8f85-494f-8f0e-22c8137563b5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"recorded_at\":\"2026-10-24T16:28:42.286Z\",\"seq\":\"16\"},\"confirmation\":null,\"issues\":[{\"field\":\"outputs.0.material_id\",\"code\":\"missing_reference\",\"message\":\"생산의 모든 행에서 품목을 선택하세요\"}]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"16\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "16"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      },
      {
        "id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.134Z",
        "created_sequence": "14",
        "corrects_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf"
      },
      {
        "id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "kind": "production",
        "created_at": "2026-10-24T16:28:42.286Z",
        "created_sequence": "16",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      },
      {
        "id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.134Z",
        "seq": "14"
      },
      {
        "id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      },
      {
        "id": "d3e859d5-5fb3-40a8-969a-f5b8456786cc",
        "record_id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "recorded_at": "2026-10-24T16:28:42.286Z",
        "seq": "16"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "current_revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      },
      {
        "record_id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "current_revision_id": "d3e859d5-5fb3-40a8-969a-f5b8456786cc"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [
      {
        "target_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "replacement_confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e"
      }
    ],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [
      {
        "id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "original_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_name": "가상 판매 메뉴",
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "selling_price": "1000",
        "known_subtotal": "500",
        "total_cost": "500",
        "cost_ratio": {
          "numerator": "500000000",
          "denominator": "1000000000"
        },
        "ratio_unavailable_reason": null,
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_versions": [
      {
        "id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": null,
        "confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "count": "3",
        "amount": "3000",
        "note": "가상 원판매",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "business_record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "count": "1",
        "amount": "1000",
        "note": "가상 판매 개수 정정",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "sale_heads": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "current_version_id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a"
      }
    ],
    "sale_cost_rows": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "0",
        "row": {
          "line_id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "numerator_quantity": "100",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "holding_average",
            "source_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
            "amount": "30000",
            "quantity": "2000",
            "reason": null
          },
          "cost": "500"
        }
      },
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "1",
        "row": {
          "line_id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "numerator_quantity": "0",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "last_valid",
            "source_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
            "amount": "30000",
            "quantity": "3000",
            "reason": null
          },
          "cost": "0"
        }
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(17,'{
  "format_version": 2,
  "schema_version": 9,
  "sequence": "17",
  "request_id": "8d82bfd4-4835-4c5b-abe1-75fb16e9f9c2",
  "recorded_at": "2026-10-24T16:28:42.349Z",
  "state": {
    "materials": [
      {
        "id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.516Z"
      },
      {
        "id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "created_at": "2026-10-24T16:28:41.761Z"
      },
      {
        "id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.887Z"
      },
      {
        "id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "created_at": "2026-10-24T16:28:41.931Z"
      }
    ],
    "material_versions": [
      {
        "id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "name": "가상 판매 곡물",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "가상 이전 자료",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.516Z",
        "seq": "1",
        "source": "catalog"
      },
      {
        "id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "name": "가상 판매 비축품",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "prepared",
        "description": "가상 실제 결과",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.761Z",
        "seq": "7",
        "source": "catalog"
      },
      {
        "id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "name": "가상 판매 메뉴",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10",
        "source": "catalog"
      },
      {
        "id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "name": "가상 판매 메뉴 (2)",
        "unit": "개",
        "canonical_unit": "개",
        "dimension": "custom",
        "kind": "menu",
        "description": "가상 레시피",
        "archived": false,
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2"
      },
      {
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22"
      }
    ],
    "material_prices": [
      {
        "id": "10953e16-bebc-4883-bce6-f603aad7c5ae",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "quantity": "1000",
        "amount": "9000",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T16:28:41.547Z",
        "seq": "2"
      }
    ],
    "catalog_state": {
      "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
      "revision": "17"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "511d56d8-252b-4a8c-863b-db419c5faabe",
        "operation": "material.create",
        "payload": "{\"request_id\":\"511d56d8-252b-4a8c-863b-db419c5faabe\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"0\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\"}",
        "response": "{\"id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"name\":\"가상 판매 곡물\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"가상 이전 자료\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.516Z\",\"updated_at\":\"2026-10-24T16:28:41.516Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"1\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "1"
      },
      {
        "request_id": "badafadd-3eca-4c2f-950d-a19229af00a7",
        "operation": "material.price",
        "payload": "[\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",{\"request_id\":\"badafadd-3eca-4c2f-950d-a19229af00a7\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"1\",\"expected_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1\",\"amount\":\"9000\",\"unit\":\"kg\"}]",
        "response": "{\"id\":\"10953e16-bebc-4883-bce6-f603aad7c5ae\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1000\",\"amount\":\"9000\",\"unit\":\"g\",\"input_quantity\":\"1\",\"input_unit\":\"kg\",\"recorded_at\":\"2026-10-24T16:28:41.547Z\",\"seq\":\"2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"2\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "2"
      },
      {
        "request_id": "68ca4836-f1dc-4b35-a6e2-ed4b150d78ff",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"2\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.580Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.580Z\",\"seq\":\"3\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"3\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "3"
      },
      {
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "operation": "business.confirm",
        "payload": "[\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",{\"meta\":{\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"3\"},\"expected_head_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"kind\":\"purchase_order\",\"head_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.580Z\",\"updated_at\":\"2026-10-24T16:28:41.608Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"effective_confirmation_id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"previous_revision_id\":\"a72cec2d-2644-4794-a84b-51900ed6f9b2\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3\",\"unit\":\"kg\",\"supplier\":\"가상 공급처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"30000\",\"note\":\"가상 주문\"}},\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\"},\"confirmation\":{\"id\":\"c2c07e97-902e-4c7f-af02-063e2183649b\",\"record_id\":\"2dbed585-ac74-470a-bd12-0aaeaa0b0e67\",\"revision_id\":\"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0\",\"request_id\":\"4dd3a072-ce75-4bfa-97cb-a2812fa7297c\",\"recorded_at\":\"2026-10-24T16:28:41.608Z\",\"seq\":\"4\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"4\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "4"
      },
      {
        "request_id": "e2051e89-2c95-4d91-b82d-f3708ebca7e9",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"e2051e89-2c95-4d91-b82d-f3708ebca7e9\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"4\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.669Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.669Z\",\"seq\":\"5\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"5\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "5"
      },
      {
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "operation": "business.confirm",
        "payload": "[\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",{\"meta\":{\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"5\"},\"expected_head_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"kind\":\"receipt\",\"head_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.669Z\",\"updated_at\":\"2026-10-24T16:28:41.715Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"effective_confirmation_id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"issue_count\":0,\"material_name\":\"가상 판매 곡물\"},\"head\":{\"id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"previous_revision_id\":\"a4121239-0040-4997-9957-5cca070bc4ff\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"order\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"expected_order_version_id\":\"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\"},\"quantity\":\"3\",\"unit\":\"kg\",\"amount\":\"30000\",\"receipt_date\":\"2026-10-01\",\"note\":\"가상 실제 입고\",\"stock_effect\":{\"kind\":\"unreflected\"}}},\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\"},\"confirmation\":{\"id\":\"45963ced-706c-4f9f-8c94-e898e6ee3b0b\",\"record_id\":\"c60fa6fa-f68f-4d3b-b976-aa8f13af603b\",\"revision_id\":\"c48b8b1c-9638-4d02-8d85-4339aa58f520\",\"request_id\":\"23c53180-c7f8-40be-8e87-a74488952588\",\"recorded_at\":\"2026-10-24T16:28:41.715Z\",\"seq\":\"6\",\"result\":{\"kind\":\"receipt\",\"receipt_id\":\"250d2cb6-f87c-4f84-a9cf-22b37ea62f39\",\"holding_version_id\":\"a1154033-8f63-4cec-9216-7d2d48fb4e5c\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"order_id\":\"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f\",\"order_version_id\":\"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"6\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "6"
      },
      {
        "request_id": "6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c",
        "operation": "material.create",
        "payload": "{\"request_id\":\"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"6\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\"}",
        "response": "{\"id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"name\":\"가상 판매 비축품\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"prepared\",\"description\":\"가상 실제 결과\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.761Z\",\"updated_at\":\"2026-10-24T16:28:41.761Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"7\"}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "7"
      },
      {
        "request_id": "4a80a87a-8a9e-46d7-a61d-7059a8abea23",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4a80a87a-8a9e-46d7-a61d-7059a8abea23\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.807Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.807Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"8\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "8"
      },
      {
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "operation": "business.confirm",
        "payload": "[\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",{\"meta\":{\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"8\"},\"expected_head_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"kind\":\"production\",\"head_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:41.807Z\",\"updated_at\":\"2026-10-24T16:28:41.853Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"effective_confirmation_id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"issue_count\":0,\"material_name\":\"가상 판매 비축품\"},\"head\":{\"id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"previous_revision_id\":\"024602c2-f983-497d-9b63-98d5f1cb7306\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"3.00\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"2\",\"unit\":\"kg\",\"allocation_percent\":\"100\"}],\"note\":\"가상 전량 생산\"}},\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"26099b3f-5312-441f-b9c2-621f39369223\",\"record_id\":\"a6dc2d71-9542-4ad2-a850-9fafcff31d0e\",\"revision_id\":\"ed4e8200-d06e-4d67-a209-322ad7f0bdd6\",\"request_id\":\"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f\",\"recorded_at\":\"2026-10-24T16:28:41.853Z\",\"seq\":\"9\",\"result\":{\"kind\":\"production\",\"production_id\":\"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"9\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "9"
      },
      {
        "request_id": "25f61e36-0c1a-4e30-a41b-51e658aab38a",
        "operation": "recipe.create",
        "payload": "{\"meta\":{\"request_id\":\"25f61e36-0c1a-4e30-a41b-51e658aab38a\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"9\"},\"owner\":{\"mode\":\"new\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"kind\":\"menu\",\"description\":\"가상 레시피\"},\"data\":{\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"quantity\":\"100.00\",\"unit\":\"g\",\"note\":\"\"},{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"0\",\"unit\":\"g\",\"note\":\"가상 적당량\"}]}}",
        "response": "{\"item\":{\"id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"name\":\"가상 판매 메뉴\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.887Z\",\"updated_at\":\"2026-10-24T16:28:41.887Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"material_name\":\"가상 판매 메뉴\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":null,\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"c1037e32-e60a-46ab-84c3-64b918d44647\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"47404a02-bf6a-44bc-872a-54061b3d0d2f\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.887Z\",\"seq\":\"10\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"10\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "10"
      },
      {
        "request_id": "9ced67a0-d991-42c2-bcfd-23230ec03eda",
        "operation": "recipe.copy",
        "payload": "[\"3c339cdf-f952-4399-bc70-a9541335a732\",{\"meta\":{\"request_id\":\"9ced67a0-d991-42c2-bcfd-23230ec03eda\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"10\"},\"expected_material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"source_recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\"}]",
        "response": "{\"item\":{\"id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"name\":\"가상 판매 메뉴 (2)\",\"unit\":\"개\",\"canonical_unit\":\"개\",\"dimension\":\"custom\",\"kind\":\"menu\",\"description\":\"가상 레시피\",\"archived\":false,\"created_at\":\"2026-10-24T16:28:41.931Z\",\"updated_at\":\"2026-10-24T16:28:41.931Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"}},\"version\":{\"id\":\"8aa2e922-b83a-4e48-a056-aa75ebfd54dd\",\"material_id\":\"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95\",\"material_version_id\":\"64ed1b2d-0b6d-4a8e-9010-6794534d7b22\",\"material_name\":\"가상 판매 메뉴 (2)\",\"material_unit\":\"개\",\"previous_version_id\":null,\"copied_from_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"basis_count\":\"3\",\"reference_output\":null,\"selling_price\":\"1000\",\"cooking_description\":\"가상 조리 설명\",\"process\":\"섞기\",\"reference_time\":\"10분\",\"lines\":[{\"id\":\"9bcf88a7-26ee-41f4-87ef-5cd339666203\",\"position\":\"0\",\"material_id\":\"2167401e-86fc-4e3a-b7d1-dbde01bf41e1\",\"material_version_id\":\"20705838-8ff0-4bcb-8260-494fabafe7c2\",\"material_name\":\"가상 판매 비축품\",\"quantity\":\"100\",\"unit\":\"g\",\"input_quantity\":\"100.00\",\"input_unit\":\"g\",\"note\":\"\"},{\"id\":\"36725a89-6820-4768-b3bf-df8c32b3799c\",\"position\":\"1\",\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"material_name\":\"가상 판매 곡물\",\"quantity\":\"0\",\"unit\":\"g\",\"input_quantity\":\"0\",\"input_unit\":\"g\",\"note\":\"가상 적당량\"}],\"recorded_at\":\"2026-10-24T16:28:41.931Z\",\"seq\":\"11\"},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"11\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "11"
      },
      {
        "request_id": "f27a90ff-74f1-4f75-aa13-b1703a77aa51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"f27a90ff-74f1-4f75-aa13-b1703a77aa51\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"11\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.009Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.009Z\",\"seq\":\"12\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"12\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "12"
      },
      {
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "operation": "business.confirm",
        "payload": "[\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",{\"meta\":{\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"12\"},\"expected_head_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"kind\":\"sale\",\"head_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.009Z\",\"updated_at\":\"2026-10-24T16:28:42.054Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"previous_revision_id\":\"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"3\",\"note\":\"가상 원판매\"}},\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\"},\"confirmation\":{\"id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"record_id\":\"0d4d5efb-2e2d-47ca-9fff-351838f2acba\",\"revision_id\":\"67a21cd0-c4c1-4cd1-b332-69a2796de1f0\",\"request_id\":\"1fb8ba7b-5772-4d53-b614-3b558359f6cd\",\"recorded_at\":\"2026-10-24T16:28:42.054Z\",\"seq\":\"13\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"61a30394-a9d3-4116-bd7e-b099d5f539ed\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"13\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "13"
      },
      {
        "request_id": "2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"13\"},\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.134Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":null,\"effective_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.134Z\",\"seq\":\"14\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"14\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "14"
      },
      {
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "operation": "business.confirm",
        "payload": "[\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",{\"meta\":{\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"14\"},\"expected_head_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"kind\":\"sale\",\"head_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T16:28:42.134Z\",\"updated_at\":\"2026-10-24T16:28:42.192Z\",\"corrects_confirmation_id\":\"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf\",\"confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"effective_confirmation_id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"issue_count\":0,\"material_name\":\"가상 판매 메뉴\"},\"head\":{\"id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"previous_revision_id\":\"df145c74-6e5a-4372-b633-cab024a4228e\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"sale\",\"version\":1,\"data\":{\"material_id\":\"3c339cdf-f952-4399-bc70-a9541335a732\",\"material_version_id\":\"f0dc73c8-4246-44d0-90c3-760ddd426515\",\"recipe_version_id\":\"e2640454-ac7e-4003-9de3-1a2145214398\",\"count\":\"1\",\"note\":\"가상 판매 개수 정정\"}},\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\"},\"confirmation\":{\"id\":\"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e\",\"record_id\":\"59800d9f-ab8b-4637-8d09-05dec46ce67e\",\"revision_id\":\"67c7d8fe-6296-43f4-9c3a-64ed31009755\",\"request_id\":\"100a2ebc-9236-44a3-98ea-1acca427c56d\",\"recorded_at\":\"2026-10-24T16:28:42.192Z\",\"seq\":\"15\",\"result\":{\"kind\":\"sale\",\"sale_id\":\"6443ff31-26b8-4442-a6f2-a760218fdcb7\",\"sale_version_id\":\"109b3cf2-0176-41dd-ba5d-a6d165b6551a\"}},\"issues\":[]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"15\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "15"
      },
      {
        "request_id": "dbd9c969-dd60-4b6b-9322-01b591afb239",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"dbd9c969-dd60-4b6b-9322-01b591afb239\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"15\"},\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"d0746729-8f85-494f-8f0e-22c8137563b5\",\"kind\":\"production\",\"head_id\":\"d3e859d5-5fb3-40a8-969a-f5b8456786cc\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.286Z\",\"updated_at\":\"2026-10-24T16:28:42.286Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":1,\"material_name\":null},\"head\":{\"id\":\"d3e859d5-5fb3-40a8-969a-f5b8456786cc\",\"record_id\":\"d0746729-8f85-494f-8f0e-22c8137563b5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"production\",\"version\":1,\"data\":{\"inputs\":[{\"material_id\":\"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81\",\"material_version_id\":\"c3348e3b-1396-4d2a-be07-2b9ecf6785de\",\"quantity\":\"1.\",\"unit\":\"kg\"}],\"outputs\":[{\"material_id\":null,\"material_version_id\":null,\"quantity\":\"\",\"unit\":\"\",\"allocation_percent\":\"\"}],\"note\":\"가상 미완성 생산\"}},\"recorded_at\":\"2026-10-24T16:28:42.286Z\",\"seq\":\"16\"},\"confirmation\":null,\"issues\":[{\"field\":\"outputs.0.material_id\",\"code\":\"missing_reference\",\"message\":\"생산의 모든 행에서 품목을 선택하세요\"}]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"16\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "16"
      },
      {
        "request_id": "8d82bfd4-4835-4c5b-abe1-75fb16e9f9c2",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"8d82bfd4-4835-4c5b-abe1-75fb16e9f9c2\",\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"expected_revision\":\"16\"},\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입고\",\"stock_effect\":{\"kind\":\"needs_reconciliation\",\"basis_id\":null,\"included_quantity\":\"\"}}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"21779c4d-d38a-4644-b033-0ed4c086ff40\",\"kind\":\"receipt\",\"head_id\":\"cd919467-794b-486f-bc95-93d2f093a951\",\"status\":\"draft\",\"created_at\":\"2026-10-24T16:28:42.349Z\",\"updated_at\":\"2026-10-24T16:28:42.349Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":6,\"material_name\":null},\"head\":{\"id\":\"cd919467-794b-486f-bc95-93d2f093a951\",\"record_id\":\"21779c4d-d38a-4644-b033-0ed4c086ff40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"receipt\",\"version\":1,\"data\":{\"source\":{\"kind\":\"direct\",\"material_id\":null,\"material_version_id\":null},\"quantity\":\"1.\",\"unit\":\"\",\"amount\":\"\",\"receipt_date\":\"\",\"note\":\"가상 미완성 입고\",\"stock_effect\":{\"kind\":\"needs_reconciliation\",\"basis_id\":null,\"included_quantity\":\"\"}}},\"recorded_at\":\"2026-10-24T16:28:42.349Z\",\"seq\":\"17\"},\"confirmation\":null,\"issues\":[{\"field\":\"stock_effect\",\"code\":\"needs_reconciliation\",\"message\":\"이미 포함되었거나 확인이 필요한 입고는 초안으로 보관합니다. 보정 연결 전에는 확정할 수 없습니다\"},{\"field\":\"source.material_id\",\"code\":\"missing_reference\",\"message\":\"입고할 재료를 선택하세요\"},{\"field\":\"quantity\",\"code\":\"invalid_quantity\",\"message\":\"소수점 뒤의 숫자를 입력하세요\"},{\"field\":\"unit\",\"code\":\"invalid_unit\",\"message\":\"관리 단위는 제어문자 없이 1~20자로 입력하세요\"},{\"field\":\"amount\",\"code\":\"invalid_amount\",\"message\":\"0 이상의 십진수와 소수 최대 6자리로 입력하세요\"},{\"field\":\"receipt_date\",\"code\":\"invalid_date\",\"message\":\"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요\"}]},\"state\":{\"epoch\":\"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0\",\"revision\":\"17\"}}",
        "epoch": "0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0",
        "sequence": "17"
      }
    ],
    "business_records": [
      {
        "id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "kind": "purchase_order",
        "created_at": "2026-10-24T16:28:41.580Z",
        "created_sequence": "3",
        "corrects_confirmation_id": null
      },
      {
        "id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:41.669Z",
        "created_sequence": "5",
        "corrects_confirmation_id": null
      },
      {
        "id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "kind": "production",
        "created_at": "2026-10-24T16:28:41.807Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.009Z",
        "created_sequence": "12",
        "corrects_confirmation_id": null
      },
      {
        "id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "kind": "sale",
        "created_at": "2026-10-24T16:28:42.134Z",
        "created_sequence": "14",
        "corrects_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf"
      },
      {
        "id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "kind": "production",
        "created_at": "2026-10-24T16:28:42.286Z",
        "created_sequence": "16",
        "corrects_confirmation_id": null
      },
      {
        "id": "21779c4d-d38a-4644-b033-0ed4c086ff40",
        "kind": "receipt",
        "created_at": "2026-10-24T16:28:42.349Z",
        "created_sequence": "17",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.580Z",
        "seq": "3"
      },
      {
        "id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "previous_revision_id": "a72cec2d-2644-4794-a84b-51900ed6f9b2",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
            "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
            "quantity": "3",
            "unit": "kg",
            "supplier": "가상 공급처",
            "expected_date": "2026-10-01",
            "estimated_amount": "30000",
            "note": "가상 주문"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.669Z",
        "seq": "5"
      },
      {
        "id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "previous_revision_id": "a4121239-0040-4997-9957-5cca070bc4ff",
        "status": "confirmed",
        "payload": {
          "kind": "receipt",
          "version": 1,
          "data": {
            "source": {
              "kind": "order",
              "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
              "expected_order_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
              "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de"
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
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.807Z",
        "seq": "8"
      },
      {
        "id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "previous_revision_id": "024602c2-f983-497d-9b63-98d5f1cb7306",
        "status": "confirmed",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
                "quantity": "3.00",
                "unit": "kg"
              }
            ],
            "outputs": [
              {
                "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
                "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
                "quantity": "2",
                "unit": "kg",
                "allocation_percent": "100"
              }
            ],
            "note": "가상 전량 생산"
          }
        },
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.009Z",
        "seq": "12"
      },
      {
        "id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "previous_revision_id": "8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "3",
            "note": "가상 원판매"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.134Z",
        "seq": "14"
      },
      {
        "id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "previous_revision_id": "df145c74-6e5a-4372-b633-cab024a4228e",
        "status": "confirmed",
        "payload": {
          "kind": "sale",
          "version": 1,
          "data": {
            "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
            "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
            "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
            "count": "1",
            "note": "가상 판매 개수 정정"
          }
        },
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      },
      {
        "id": "d3e859d5-5fb3-40a8-969a-f5b8456786cc",
        "record_id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "production",
          "version": 1,
          "data": {
            "inputs": [
              {
                "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
                "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "recorded_at": "2026-10-24T16:28:42.286Z",
        "seq": "16"
      },
      {
        "id": "cd919467-794b-486f-bc95-93d2f093a951",
        "record_id": "21779c4d-d38a-4644-b033-0ed4c086ff40",
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
        "recorded_at": "2026-10-24T16:28:42.349Z",
        "seq": "17"
      }
    ],
    "business_heads": [
      {
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "current_revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0"
      },
      {
        "record_id": "21779c4d-d38a-4644-b033-0ed4c086ff40",
        "current_revision_id": "cd919467-794b-486f-bc95-93d2f093a951"
      },
      {
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "current_revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0"
      },
      {
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "current_revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755"
      },
      {
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "current_revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6"
      },
      {
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "current_revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520"
      },
      {
        "record_id": "d0746729-8f85-494f-8f0e-22c8137563b5",
        "current_revision_id": "d3e859d5-5fb3-40a8-969a-f5b8456786cc"
      }
    ],
    "business_confirmations": [
      {
        "id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "revision_id": "fc716d0d-75ac-4730-aad1-acc9ec1ce9f0",
        "request_id": "4dd3a072-ce75-4bfa-97cb-a2812fa7297c",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "revision_id": "c48b8b1c-9638-4d02-8d85-4339aa58f520",
        "request_id": "23c53180-c7f8-40be-8e87-a74488952588",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "26099b3f-5312-441f-b9c2-621f39369223",
        "record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "revision_id": "ed4e8200-d06e-4d67-a209-322ad7f0bdd6",
        "request_id": "f88ce438-fc06-4cf5-9b03-fcc0b81ec54f",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "revision_id": "67a21cd0-c4c1-4cd1-b332-69a2796de1f0",
        "request_id": "1fb8ba7b-5772-4d53-b614-3b558359f6cd",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "revision_id": "67c7d8fe-6296-43f4-9c3a-64ed31009755",
        "request_id": "100a2ebc-9236-44a3-98ea-1acca427c56d",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "business_catalog_price_results": [],
    "business_corrections": [
      {
        "target_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "replacement_confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e"
      }
    ],
    "purchase_orders": [
      {
        "id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "first_confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "created_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": null,
        "confirmation_id": "c2c07e97-902e-4c7f-af02-063e2183649b",
        "business_record_id": "2dbed585-ac74-470a-bd12-0aaeaa0b0e67",
        "action": "place",
        "status": "open",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.608Z",
        "seq": "4"
      },
      {
        "id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "previous_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "action": "receive",
        "status": "received",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "supplier": "가상 공급처",
        "expected_date": "2026-10-01",
        "estimated_amount": "30000",
        "note": "가상 주문",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "current_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"
      }
    ],
    "receipts": [
      {
        "id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "business_record_id": "c60fa6fa-f68f-4d3b-b976-aa8f13af603b",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "material_name": "가상 판매 곡물",
        "quantity": "3000",
        "unit": "g",
        "input_quantity": "3",
        "input_unit": "kg",
        "amount": "30000",
        "receipt_date": "2026-10-01",
        "note": "가상 실제 입고",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6",
        "source": {
          "kind": "order",
          "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
          "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f"
        }
      }
    ],
    "holding_versions": [
      {
        "id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": null,
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": null,
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "3000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.715Z",
        "seq": "6"
      },
      {
        "id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "previous_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
        "receipt_id": null,
        "baseline_receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "production_effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "baseline_production_effect_id": null,
        "baseline_seq": "6",
        "quantity": "0",
        "value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "previous_version_id": null,
        "receipt_id": null,
        "baseline_receipt_id": null,
        "production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_production_effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "baseline_seq": "9",
        "quantity": "2000",
        "value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "holding_heads": [
      {
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "current_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
        "last_valid_version_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17"
      },
      {
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "current_version_id": "aea60879-d29d-4bfc-8ec3-5c3fde5a6494",
        "last_valid_version_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c"
      }
    ],
    "order_receipt_closures": [
      {
        "order_id": "e75a6474-ff25-4b00-a067-5a7fc5ed9f9f",
        "receipt_id": "250d2cb6-f87c-4f84-a9cf-22b37ea62f39",
        "confirmation_id": "45963ced-706c-4f9f-8c94-e898e6ee3b0b",
        "initial_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "ordered_version_id": "3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f",
        "received_version_id": "ae5c71ed-65c3-4d80-ac40-51ae2c37c19f",
        "received_quantity": "3000",
        "difference": "0"
      }
    ],
    "recipe_versions": [
      {
        "id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
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
        "recorded_at": "2026-10-24T16:28:41.887Z",
        "seq": "10"
      },
      {
        "id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "material_version_id": "64ed1b2d-0b6d-4a8e-9010-6794534d7b22",
        "material_name": "가상 판매 메뉴 (2)",
        "material_unit": "개",
        "previous_version_id": null,
        "copied_from_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "reference_output": null,
        "selling_price": "1000",
        "cooking_description": "가상 조리 설명",
        "process": "섞기",
        "reference_time": "10분",
        "recorded_at": "2026-10-24T16:28:41.931Z",
        "seq": "11"
      }
    ],
    "recipe_lines": [
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "9bcf88a7-26ee-41f4-87ef-5cd339666203",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd",
        "line": {
          "id": "36725a89-6820-4768-b3bf-df8c32b3799c",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "quantity": "0",
          "unit": "g",
          "input_quantity": "0",
          "input_unit": "g",
          "note": "가상 적당량"
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "quantity": "100",
          "unit": "g",
          "input_quantity": "100.00",
          "input_unit": "g",
          "note": ""
        }
      },
      {
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "line": {
          "id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "position": "1",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "current_version_id": "e2640454-ac7e-4003-9de3-1a2145214398"
      },
      {
        "material_id": "8c6d323c-0ff3-4b48-a7bb-ebca8616dd95",
        "current_version_id": "8aa2e922-b83a-4e48-a056-aa75ebfd54dd"
      }
    ],
    "productions": [
      {
        "id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "confirmation_id": "26099b3f-5312-441f-b9c2-621f39369223",
        "business_record_id": "a6dc2d71-9542-4ad2-a850-9fafcff31d0e",
        "note": "가상 전량 생산",
        "total_input_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "production_inputs": [
      {
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "row": {
          "position": "0",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
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
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "effect_id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "row": {
          "position": "0",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
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
        "id": "a05297d7-3d72-4ffa-a453-8a414c6f2c36",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
        "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
        "input_quantity": "3000",
        "input_value": "30000",
        "output_quantity": "0",
        "output_value": "0",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      },
      {
        "id": "a6fdd716-d5a8-40d9-92b3-87d4cd4149cc",
        "production_id": "6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a",
        "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
        "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
        "input_quantity": "0",
        "input_value": "0",
        "output_quantity": "2000",
        "output_value": "30000",
        "recorded_at": "2026-10-24T16:28:41.853Z",
        "seq": "9"
      }
    ],
    "sales": [
      {
        "id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "original_confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "material_id": "3c339cdf-f952-4399-bc70-a9541335a732",
        "material_version_id": "f0dc73c8-4246-44d0-90c3-760ddd426515",
        "material_name": "가상 판매 메뉴",
        "recipe_version_id": "e2640454-ac7e-4003-9de3-1a2145214398",
        "basis_count": "3",
        "selling_price": "1000",
        "known_subtotal": "500",
        "total_cost": "500",
        "cost_ratio": {
          "numerator": "500000000",
          "denominator": "1000000000"
        },
        "ratio_unavailable_reason": null,
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      }
    ],
    "sale_versions": [
      {
        "id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": null,
        "confirmation_id": "a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf",
        "business_record_id": "0d4d5efb-2e2d-47ca-9fff-351838f2acba",
        "count": "3",
        "amount": "3000",
        "note": "가상 원판매",
        "recorded_at": "2026-10-24T16:28:42.054Z",
        "seq": "13"
      },
      {
        "id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a",
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "previous_version_id": "61a30394-a9d3-4116-bd7e-b099d5f539ed",
        "confirmation_id": "3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e",
        "business_record_id": "59800d9f-ab8b-4637-8d09-05dec46ce67e",
        "count": "1",
        "amount": "1000",
        "note": "가상 판매 개수 정정",
        "recorded_at": "2026-10-24T16:28:42.192Z",
        "seq": "15"
      }
    ],
    "sale_heads": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "current_version_id": "109b3cf2-0176-41dd-ba5d-a6d165b6551a"
      }
    ],
    "sale_cost_rows": [
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "0",
        "row": {
          "line_id": "c1037e32-e60a-46ab-84c3-64b918d44647",
          "material_id": "2167401e-86fc-4e3a-b7d1-dbde01bf41e1",
          "material_version_id": "20705838-8ff0-4bcb-8260-494fabafe7c2",
          "material_name": "가상 판매 비축품",
          "numerator_quantity": "100",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "holding_average",
            "source_id": "cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17",
            "amount": "30000",
            "quantity": "2000",
            "reason": null
          },
          "cost": "500"
        }
      },
      {
        "sale_id": "6443ff31-26b8-4442-a6f2-a760218fdcb7",
        "position": "1",
        "row": {
          "line_id": "47404a02-bf6a-44bc-872a-54061b3d0d2f",
          "material_id": "5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",
          "material_version_id": "c3348e3b-1396-4d2a-be07-2b9ecf6785de",
          "material_name": "가상 판매 곡물",
          "numerator_quantity": "0",
          "basis_count": "3",
          "unit": "g",
          "price": {
            "source": "last_valid",
            "source_id": "a1154033-8f63-4cec-9216-7d2d48fb4e5c",
            "amount": "30000",
            "quantity": "3000",
            "reason": null
          },
          "cost": "0"
        }
      }
    ]
  }
}
',1);
INSERT INTO "material_versions" VALUES('c3348e3b-1396-4d2a-be07-2b9ecf6785de','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','가상 판매 곡물','g','g','mass','raw','가상 이전 자료',0,'2026-10-24T16:28:41.516Z',1,'catalog');
INSERT INTO "material_versions" VALUES('20705838-8ff0-4bcb-8260-494fabafe7c2','2167401e-86fc-4e3a-b7d1-dbde01bf41e1','가상 판매 비축품','g','g','mass','prepared','가상 실제 결과',0,'2026-10-24T16:28:41.761Z',7,'catalog');
INSERT INTO "material_versions" VALUES('f0dc73c8-4246-44d0-90c3-760ddd426515','3c339cdf-f952-4399-bc70-a9541335a732','가상 판매 메뉴','개','개','custom','menu','가상 레시피',0,'2026-10-24T16:28:41.887Z',10,'catalog');
INSERT INTO "material_versions" VALUES('64ed1b2d-0b6d-4a8e-9010-6794534d7b22','8c6d323c-0ff3-4b48-a7bb-ebca8616dd95','가상 판매 메뉴 (2)','개','개','custom','menu','가상 레시피',0,'2026-10-24T16:28:41.931Z',11,'catalog');
INSERT INTO "material_heads" VALUES('5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de');
INSERT INTO "material_heads" VALUES('2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2');
INSERT INTO "material_heads" VALUES('3c339cdf-f952-4399-bc70-a9541335a732','f0dc73c8-4246-44d0-90c3-760ddd426515');
INSERT INTO "material_heads" VALUES('8c6d323c-0ff3-4b48-a7bb-ebca8616dd95','64ed1b2d-0b6d-4a8e-9010-6794534d7b22');
INSERT INTO "material_prices" VALUES('10953e16-bebc-4883-bce6-f603aad7c5ae','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',1000000000,9000000000,'g','1','kg','2026-10-24T16:28:41.547Z',2);
INSERT INTO "mutation_requests" VALUES('511d56d8-252b-4a8c-863b-db419c5faabe','material.create','{"request_id":"511d56d8-252b-4a8c-863b-db419c5faabe","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"0","name":"가상 판매 곡물","unit":"g","kind":"raw","description":"가상 이전 자료"}','{"id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","name":"가상 판매 곡물","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"가상 이전 자료","archived":false,"created_at":"2026-10-24T16:28:41.516Z","updated_at":"2026-10-24T16:28:41.516Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"1"}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',1);
INSERT INTO "mutation_requests" VALUES('badafadd-3eca-4c2f-950d-a19229af00a7','material.price','["5c7aae45-c1bb-41c0-a205-4ddeaa5cba81",{"request_id":"badafadd-3eca-4c2f-950d-a19229af00a7","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"1","expected_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"1","amount":"9000","unit":"kg"}]','{"id":"10953e16-bebc-4883-bce6-f603aad7c5ae","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"1000","amount":"9000","unit":"g","input_quantity":"1","input_unit":"kg","recorded_at":"2026-10-24T16:28:41.547Z","seq":"2","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"2"}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',2);
INSERT INTO "mutation_requests" VALUES('68ca4836-f1dc-4b35-a6e2-ed4b150d78ff','business.create','{"meta":{"request_id":"68ca4836-f1dc-4b35-a6e2-ed4b150d78ff","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"2"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"2dbed585-ac74-470a-bd12-0aaeaa0b0e67","kind":"purchase_order","head_id":"a72cec2d-2644-4794-a84b-51900ed6f9b2","status":"draft","created_at":"2026-10-24T16:28:41.580Z","updated_at":"2026-10-24T16:28:41.580Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"a72cec2d-2644-4794-a84b-51900ed6f9b2","record_id":"2dbed585-ac74-470a-bd12-0aaeaa0b0e67","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T16:28:41.580Z","seq":"3"},"confirmation":null,"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"3"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',3);
INSERT INTO "mutation_requests" VALUES('4dd3a072-ce75-4bfa-97cb-a2812fa7297c','business.confirm','["2dbed585-ac74-470a-bd12-0aaeaa0b0e67",{"meta":{"request_id":"4dd3a072-ce75-4bfa-97cb-a2812fa7297c","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"3"},"expected_head_id":"a72cec2d-2644-4794-a84b-51900ed6f9b2"}]','{"result":{"item":{"id":"2dbed585-ac74-470a-bd12-0aaeaa0b0e67","kind":"purchase_order","head_id":"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0","status":"confirmed","created_at":"2026-10-24T16:28:41.580Z","updated_at":"2026-10-24T16:28:41.608Z","corrects_confirmation_id":null,"confirmation_id":"c2c07e97-902e-4c7f-af02-063e2183649b","effective_confirmation_id":"c2c07e97-902e-4c7f-af02-063e2183649b","issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0","record_id":"2dbed585-ac74-470a-bd12-0aaeaa0b0e67","previous_revision_id":"a72cec2d-2644-4794-a84b-51900ed6f9b2","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}},"recorded_at":"2026-10-24T16:28:41.608Z","seq":"4"},"confirmation":{"id":"c2c07e97-902e-4c7f-af02-063e2183649b","record_id":"2dbed585-ac74-470a-bd12-0aaeaa0b0e67","revision_id":"fc716d0d-75ac-4730-aad1-acc9ec1ce9f0","request_id":"4dd3a072-ce75-4bfa-97cb-a2812fa7297c","recorded_at":"2026-10-24T16:28:41.608Z","seq":"4","result":{"kind":"purchase_order","action":"place","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"}},"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"4"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',4);
INSERT INTO "mutation_requests" VALUES('e2051e89-2c95-4d91-b82d-f3708ebca7e9','business.create','{"meta":{"request_id":"e2051e89-2c95-4d91-b82d-f3708ebca7e9","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"4"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","expected_order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"c60fa6fa-f68f-4d3b-b976-aa8f13af603b","kind":"receipt","head_id":"a4121239-0040-4997-9957-5cca070bc4ff","status":"draft","created_at":"2026-10-24T16:28:41.669Z","updated_at":"2026-10-24T16:28:41.669Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"a4121239-0040-4997-9957-5cca070bc4ff","record_id":"c60fa6fa-f68f-4d3b-b976-aa8f13af603b","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","expected_order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T16:28:41.669Z","seq":"5"},"confirmation":null,"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"5"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',5);
INSERT INTO "mutation_requests" VALUES('23c53180-c7f8-40be-8e87-a74488952588','business.confirm','["c60fa6fa-f68f-4d3b-b976-aa8f13af603b",{"meta":{"request_id":"23c53180-c7f8-40be-8e87-a74488952588","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"5"},"expected_head_id":"a4121239-0040-4997-9957-5cca070bc4ff"}]','{"result":{"item":{"id":"c60fa6fa-f68f-4d3b-b976-aa8f13af603b","kind":"receipt","head_id":"c48b8b1c-9638-4d02-8d85-4339aa58f520","status":"confirmed","created_at":"2026-10-24T16:28:41.669Z","updated_at":"2026-10-24T16:28:41.715Z","corrects_confirmation_id":null,"confirmation_id":"45963ced-706c-4f9f-8c94-e898e6ee3b0b","effective_confirmation_id":"45963ced-706c-4f9f-8c94-e898e6ee3b0b","issue_count":0,"material_name":"가상 판매 곡물"},"head":{"id":"c48b8b1c-9638-4d02-8d85-4339aa58f520","record_id":"c60fa6fa-f68f-4d3b-b976-aa8f13af603b","previous_revision_id":"a4121239-0040-4997-9957-5cca070bc4ff","status":"confirmed","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","expected_order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}},"recorded_at":"2026-10-24T16:28:41.715Z","seq":"6"},"confirmation":{"id":"45963ced-706c-4f9f-8c94-e898e6ee3b0b","record_id":"c60fa6fa-f68f-4d3b-b976-aa8f13af603b","revision_id":"c48b8b1c-9638-4d02-8d85-4339aa58f520","request_id":"23c53180-c7f8-40be-8e87-a74488952588","recorded_at":"2026-10-24T16:28:41.715Z","seq":"6","result":{"kind":"receipt","receipt_id":"250d2cb6-f87c-4f84-a9cf-22b37ea62f39","holding_version_id":"a1154033-8f63-4cec-9216-7d2d48fb4e5c","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","order_version_id":"ae5c71ed-65c3-4d80-ac40-51ae2c37c19f"}},"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"6"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',6);
INSERT INTO "mutation_requests" VALUES('6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c','material.create','{"request_id":"6a22e88f-dff5-42f4-8829-d0d9f8f4dc9c","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"6","name":"가상 판매 비축품","unit":"g","kind":"prepared","description":"가상 실제 결과"}','{"id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","name":"가상 판매 비축품","unit":"g","canonical_unit":"g","dimension":"mass","kind":"prepared","description":"가상 실제 결과","archived":false,"created_at":"2026-10-24T16:28:41.761Z","updated_at":"2026-10-24T16:28:41.761Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"7"}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',7);
INSERT INTO "mutation_requests" VALUES('4a80a87a-8a9e-46d7-a61d-7059a8abea23','business.create','{"meta":{"request_id":"4a80a87a-8a9e-46d7-a61d-7059a8abea23","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"7"},"payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"a6dc2d71-9542-4ad2-a850-9fafcff31d0e","kind":"production","head_id":"024602c2-f983-497d-9b63-98d5f1cb7306","status":"draft","created_at":"2026-10-24T16:28:41.807Z","updated_at":"2026-10-24T16:28:41.807Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 비축품"},"head":{"id":"024602c2-f983-497d-9b63-98d5f1cb7306","record_id":"a6dc2d71-9542-4ad2-a850-9fafcff31d0e","previous_revision_id":null,"status":"draft","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"recorded_at":"2026-10-24T16:28:41.807Z","seq":"8"},"confirmation":null,"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"8"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',8);
INSERT INTO "mutation_requests" VALUES('f88ce438-fc06-4cf5-9b03-fcc0b81ec54f','business.confirm','["a6dc2d71-9542-4ad2-a850-9fafcff31d0e",{"meta":{"request_id":"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"8"},"expected_head_id":"024602c2-f983-497d-9b63-98d5f1cb7306"}]','{"result":{"item":{"id":"a6dc2d71-9542-4ad2-a850-9fafcff31d0e","kind":"production","head_id":"ed4e8200-d06e-4d67-a209-322ad7f0bdd6","status":"confirmed","created_at":"2026-10-24T16:28:41.807Z","updated_at":"2026-10-24T16:28:41.853Z","corrects_confirmation_id":null,"confirmation_id":"26099b3f-5312-441f-b9c2-621f39369223","effective_confirmation_id":"26099b3f-5312-441f-b9c2-621f39369223","issue_count":0,"material_name":"가상 판매 비축품"},"head":{"id":"ed4e8200-d06e-4d67-a209-322ad7f0bdd6","record_id":"a6dc2d71-9542-4ad2-a850-9fafcff31d0e","previous_revision_id":"024602c2-f983-497d-9b63-98d5f1cb7306","status":"confirmed","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}},"recorded_at":"2026-10-24T16:28:41.853Z","seq":"9"},"confirmation":{"id":"26099b3f-5312-441f-b9c2-621f39369223","record_id":"a6dc2d71-9542-4ad2-a850-9fafcff31d0e","revision_id":"ed4e8200-d06e-4d67-a209-322ad7f0bdd6","request_id":"f88ce438-fc06-4cf5-9b03-fcc0b81ec54f","recorded_at":"2026-10-24T16:28:41.853Z","seq":"9","result":{"kind":"production","production_id":"6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a"}},"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"9"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',9);
INSERT INTO "mutation_requests" VALUES('25f61e36-0c1a-4e30-a41b-51e658aab38a','recipe.create','{"meta":{"request_id":"25f61e36-0c1a-4e30-a41b-51e658aab38a","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"9"},"owner":{"mode":"new","name":"가상 판매 메뉴","unit":"개","kind":"menu","description":"가상 레시피"},"data":{"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"100.00","unit":"g","note":""},{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"0","unit":"g","note":"가상 적당량"}]}}','{"item":{"id":"3c339cdf-f952-4399-bc70-a9541335a732","version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","name":"가상 판매 메뉴","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T16:28:41.887Z","updated_at":"2026-10-24T16:28:41.887Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"e2640454-ac7e-4003-9de3-1a2145214398","material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","material_name":"가상 판매 메뉴","material_unit":"개","previous_version_id":null,"copied_from_version_id":null,"basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"c1037e32-e60a-46ab-84c3-64b918d44647","position":"0","material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","material_name":"가상 판매 비축품","quantity":"100","unit":"g","input_quantity":"100.00","input_unit":"g","note":""},{"id":"47404a02-bf6a-44bc-872a-54061b3d0d2f","position":"1","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","material_name":"가상 판매 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T16:28:41.887Z","seq":"10"},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"10"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',10);
INSERT INTO "mutation_requests" VALUES('9ced67a0-d991-42c2-bcfd-23230ec03eda','recipe.copy','["3c339cdf-f952-4399-bc70-a9541335a732",{"meta":{"request_id":"9ced67a0-d991-42c2-bcfd-23230ec03eda","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"10"},"expected_material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","source_recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398"}]','{"item":{"id":"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95","version_id":"64ed1b2d-0b6d-4a8e-9010-6794534d7b22","name":"가상 판매 메뉴 (2)","unit":"개","canonical_unit":"개","dimension":"custom","kind":"menu","description":"가상 레시피","archived":false,"created_at":"2026-10-24T16:28:41.931Z","updated_at":"2026-10-24T16:28:41.931Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"}},"version":{"id":"8aa2e922-b83a-4e48-a056-aa75ebfd54dd","material_id":"8c6d323c-0ff3-4b48-a7bb-ebca8616dd95","material_version_id":"64ed1b2d-0b6d-4a8e-9010-6794534d7b22","material_name":"가상 판매 메뉴 (2)","material_unit":"개","previous_version_id":null,"copied_from_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","basis_count":"3","reference_output":null,"selling_price":"1000","cooking_description":"가상 조리 설명","process":"섞기","reference_time":"10분","lines":[{"id":"9bcf88a7-26ee-41f4-87ef-5cd339666203","position":"0","material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","material_name":"가상 판매 비축품","quantity":"100","unit":"g","input_quantity":"100.00","input_unit":"g","note":""},{"id":"36725a89-6820-4768-b3bf-df8c32b3799c","position":"1","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","material_name":"가상 판매 곡물","quantity":"0","unit":"g","input_quantity":"0","input_unit":"g","note":"가상 적당량"}],"recorded_at":"2026-10-24T16:28:41.931Z","seq":"11"},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"11"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',11);
INSERT INTO "mutation_requests" VALUES('f27a90ff-74f1-4f75-aa13-b1703a77aa51','business.create','{"meta":{"request_id":"f27a90ff-74f1-4f75-aa13-b1703a77aa51","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"11"},"payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"3","note":"가상 원판매"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"0d4d5efb-2e2d-47ca-9fff-351838f2acba","kind":"sale","head_id":"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b","status":"draft","created_at":"2026-10-24T16:28:42.009Z","updated_at":"2026-10-24T16:28:42.009Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 판매 메뉴"},"head":{"id":"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b","record_id":"0d4d5efb-2e2d-47ca-9fff-351838f2acba","previous_revision_id":null,"status":"draft","payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"3","note":"가상 원판매"}},"recorded_at":"2026-10-24T16:28:42.009Z","seq":"12"},"confirmation":null,"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"12"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',12);
INSERT INTO "mutation_requests" VALUES('1fb8ba7b-5772-4d53-b614-3b558359f6cd','business.confirm','["0d4d5efb-2e2d-47ca-9fff-351838f2acba",{"meta":{"request_id":"1fb8ba7b-5772-4d53-b614-3b558359f6cd","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"12"},"expected_head_id":"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b"}]','{"result":{"item":{"id":"0d4d5efb-2e2d-47ca-9fff-351838f2acba","kind":"sale","head_id":"67a21cd0-c4c1-4cd1-b332-69a2796de1f0","status":"confirmed","created_at":"2026-10-24T16:28:42.009Z","updated_at":"2026-10-24T16:28:42.054Z","corrects_confirmation_id":null,"confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","effective_confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","issue_count":0,"material_name":"가상 판매 메뉴"},"head":{"id":"67a21cd0-c4c1-4cd1-b332-69a2796de1f0","record_id":"0d4d5efb-2e2d-47ca-9fff-351838f2acba","previous_revision_id":"8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b","status":"confirmed","payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"3","note":"가상 원판매"}},"recorded_at":"2026-10-24T16:28:42.054Z","seq":"13"},"confirmation":{"id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","record_id":"0d4d5efb-2e2d-47ca-9fff-351838f2acba","revision_id":"67a21cd0-c4c1-4cd1-b332-69a2796de1f0","request_id":"1fb8ba7b-5772-4d53-b614-3b558359f6cd","recorded_at":"2026-10-24T16:28:42.054Z","seq":"13","result":{"kind":"sale","sale_id":"6443ff31-26b8-4442-a6f2-a760218fdcb7","sale_version_id":"61a30394-a9d3-4116-bd7e-b099d5f539ed"}},"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"13"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',13);
INSERT INTO "mutation_requests" VALUES('2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6','business.create','{"meta":{"request_id":"2edc2fb1-6dc2-4bd5-b853-6f40da85a9d6","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"13"},"payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"1","note":"가상 판매 개수 정정"}},"corrects_confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf"}','{"result":{"item":{"id":"59800d9f-ab8b-4637-8d09-05dec46ce67e","kind":"sale","head_id":"df145c74-6e5a-4372-b633-cab024a4228e","status":"draft","created_at":"2026-10-24T16:28:42.134Z","updated_at":"2026-10-24T16:28:42.134Z","corrects_confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","confirmation_id":null,"effective_confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","issue_count":0,"material_name":"가상 판매 메뉴"},"head":{"id":"df145c74-6e5a-4372-b633-cab024a4228e","record_id":"59800d9f-ab8b-4637-8d09-05dec46ce67e","previous_revision_id":null,"status":"draft","payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"1","note":"가상 판매 개수 정정"}},"recorded_at":"2026-10-24T16:28:42.134Z","seq":"14"},"confirmation":null,"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"14"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',14);
INSERT INTO "mutation_requests" VALUES('100a2ebc-9236-44a3-98ea-1acca427c56d','business.confirm','["59800d9f-ab8b-4637-8d09-05dec46ce67e",{"meta":{"request_id":"100a2ebc-9236-44a3-98ea-1acca427c56d","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"14"},"expected_head_id":"df145c74-6e5a-4372-b633-cab024a4228e"}]','{"result":{"item":{"id":"59800d9f-ab8b-4637-8d09-05dec46ce67e","kind":"sale","head_id":"67c7d8fe-6296-43f4-9c3a-64ed31009755","status":"confirmed","created_at":"2026-10-24T16:28:42.134Z","updated_at":"2026-10-24T16:28:42.192Z","corrects_confirmation_id":"a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf","confirmation_id":"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e","effective_confirmation_id":"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e","issue_count":0,"material_name":"가상 판매 메뉴"},"head":{"id":"67c7d8fe-6296-43f4-9c3a-64ed31009755","record_id":"59800d9f-ab8b-4637-8d09-05dec46ce67e","previous_revision_id":"df145c74-6e5a-4372-b633-cab024a4228e","status":"confirmed","payload":{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"1","note":"가상 판매 개수 정정"}},"recorded_at":"2026-10-24T16:28:42.192Z","seq":"15"},"confirmation":{"id":"3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e","record_id":"59800d9f-ab8b-4637-8d09-05dec46ce67e","revision_id":"67c7d8fe-6296-43f4-9c3a-64ed31009755","request_id":"100a2ebc-9236-44a3-98ea-1acca427c56d","recorded_at":"2026-10-24T16:28:42.192Z","seq":"15","result":{"kind":"sale","sale_id":"6443ff31-26b8-4442-a6f2-a760218fdcb7","sale_version_id":"109b3cf2-0176-41dd-ba5d-a6d165b6551a"}},"issues":[]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"15"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',15);
INSERT INTO "mutation_requests" VALUES('dbd9c969-dd60-4b6b-9322-01b591afb239','business.create','{"meta":{"request_id":"dbd9c969-dd60-4b6b-9322-01b591afb239","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"15"},"payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"d0746729-8f85-494f-8f0e-22c8137563b5","kind":"production","head_id":"d3e859d5-5fb3-40a8-969a-f5b8456786cc","status":"draft","created_at":"2026-10-24T16:28:42.286Z","updated_at":"2026-10-24T16:28:42.286Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":1,"material_name":null},"head":{"id":"d3e859d5-5fb3-40a8-969a-f5b8456786cc","record_id":"d0746729-8f85-494f-8f0e-22c8137563b5","previous_revision_id":null,"status":"draft","payload":{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}},"recorded_at":"2026-10-24T16:28:42.286Z","seq":"16"},"confirmation":null,"issues":[{"field":"outputs.0.material_id","code":"missing_reference","message":"생산의 모든 행에서 품목을 선택하세요"}]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"16"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',16);
INSERT INTO "mutation_requests" VALUES('8d82bfd4-4835-4c5b-abe1-75fb16e9f9c2','business.create','{"meta":{"request_id":"8d82bfd4-4835-4c5b-abe1-75fb16e9f9c2","epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","expected_revision":"16"},"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"21779c4d-d38a-4644-b033-0ed4c086ff40","kind":"receipt","head_id":"cd919467-794b-486f-bc95-93d2f093a951","status":"draft","created_at":"2026-10-24T16:28:42.349Z","updated_at":"2026-10-24T16:28:42.349Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":6,"material_name":null},"head":{"id":"cd919467-794b-486f-bc95-93d2f093a951","record_id":"21779c4d-d38a-4644-b033-0ed4c086ff40","previous_revision_id":null,"status":"draft","payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}},"recorded_at":"2026-10-24T16:28:42.349Z","seq":"17"},"confirmation":null,"issues":[{"field":"stock_effect","code":"needs_reconciliation","message":"이미 포함되었거나 확인이 필요한 입고는 초안으로 보관합니다. 보정 연결 전에는 확정할 수 없습니다"},{"field":"source.material_id","code":"missing_reference","message":"입고할 재료를 선택하세요"},{"field":"quantity","code":"invalid_quantity","message":"소수점 뒤의 숫자를 입력하세요"},{"field":"unit","code":"invalid_unit","message":"관리 단위는 제어문자 없이 1~20자로 입력하세요"},{"field":"amount","code":"invalid_amount","message":"0 이상의 십진수와 소수 최대 6자리로 입력하세요"},{"field":"receipt_date","code":"invalid_date","message":"입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요"}]},"state":{"epoch":"0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0","revision":"17"}}','0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',17);
INSERT INTO "catalog_state" VALUES(1,'0ef3c2ff-e923-4c1a-89f5-602e4ff4a7a0',17);
INSERT INTO "business_records" VALUES('2dbed585-ac74-470a-bd12-0aaeaa0b0e67','purchase_order','2026-10-24T16:28:41.580Z',3,NULL);
INSERT INTO "business_records" VALUES('c60fa6fa-f68f-4d3b-b976-aa8f13af603b','receipt','2026-10-24T16:28:41.669Z',5,NULL);
INSERT INTO "business_records" VALUES('a6dc2d71-9542-4ad2-a850-9fafcff31d0e','production','2026-10-24T16:28:41.807Z',8,NULL);
INSERT INTO "business_records" VALUES('0d4d5efb-2e2d-47ca-9fff-351838f2acba','sale','2026-10-24T16:28:42.009Z',12,NULL);
INSERT INTO "business_records" VALUES('59800d9f-ab8b-4637-8d09-05dec46ce67e','sale','2026-10-24T16:28:42.134Z',14,'a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf');
INSERT INTO "business_records" VALUES('d0746729-8f85-494f-8f0e-22c8137563b5','production','2026-10-24T16:28:42.286Z',16,NULL);
INSERT INTO "business_records" VALUES('21779c4d-d38a-4644-b033-0ed4c086ff40','receipt','2026-10-24T16:28:42.349Z',17,NULL);
INSERT INTO "business_revisions" VALUES('a72cec2d-2644-4794-a84b-51900ed6f9b2','2dbed585-ac74-470a-bd12-0aaeaa0b0e67',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T16:28:41.580Z',3);
INSERT INTO "business_revisions" VALUES('fc716d0d-75ac-4730-aad1-acc9ec1ce9f0','2dbed585-ac74-470a-bd12-0aaeaa0b0e67','a72cec2d-2644-4794-a84b-51900ed6f9b2','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3","unit":"kg","supplier":"가상 공급처","expected_date":"2026-10-01","estimated_amount":"30000","note":"가상 주문"}}','2026-10-24T16:28:41.608Z',4);
INSERT INTO "business_revisions" VALUES('a4121239-0040-4997-9957-5cca070bc4ff','c60fa6fa-f68f-4d3b-b976-aa8f13af603b',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","expected_order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T16:28:41.669Z',5);
INSERT INTO "business_revisions" VALUES('c48b8b1c-9638-4d02-8d85-4339aa58f520','c60fa6fa-f68f-4d3b-b976-aa8f13af603b','a4121239-0040-4997-9957-5cca070bc4ff','confirmed',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"order","order_id":"e75a6474-ff25-4b00-a067-5a7fc5ed9f9f","expected_order_version_id":"3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de"},"quantity":"3","unit":"kg","amount":"30000","receipt_date":"2026-10-01","note":"가상 실제 입고","stock_effect":{"kind":"unreflected"}}}','2026-10-24T16:28:41.715Z',6);
INSERT INTO "business_revisions" VALUES('024602c2-f983-497d-9b63-98d5f1cb7306','a6dc2d71-9542-4ad2-a850-9fafcff31d0e',NULL,'draft',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}}','2026-10-24T16:28:41.807Z',8);
INSERT INTO "business_revisions" VALUES('ed4e8200-d06e-4d67-a209-322ad7f0bdd6','a6dc2d71-9542-4ad2-a850-9fafcff31d0e','024602c2-f983-497d-9b63-98d5f1cb7306','confirmed',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"3.00","unit":"kg"}],"outputs":[{"material_id":"2167401e-86fc-4e3a-b7d1-dbde01bf41e1","material_version_id":"20705838-8ff0-4bcb-8260-494fabafe7c2","quantity":"2","unit":"kg","allocation_percent":"100"}],"note":"가상 전량 생산"}}','2026-10-24T16:28:41.853Z',9);
INSERT INTO "business_revisions" VALUES('8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b','0d4d5efb-2e2d-47ca-9fff-351838f2acba',NULL,'draft',1,'{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"3","note":"가상 원판매"}}','2026-10-24T16:28:42.009Z',12);
INSERT INTO "business_revisions" VALUES('67a21cd0-c4c1-4cd1-b332-69a2796de1f0','0d4d5efb-2e2d-47ca-9fff-351838f2acba','8fb119cf-f72f-4e0d-9d2c-0a92b5cf234b','confirmed',1,'{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"3","note":"가상 원판매"}}','2026-10-24T16:28:42.054Z',13);
INSERT INTO "business_revisions" VALUES('df145c74-6e5a-4372-b633-cab024a4228e','59800d9f-ab8b-4637-8d09-05dec46ce67e',NULL,'draft',1,'{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"1","note":"가상 판매 개수 정정"}}','2026-10-24T16:28:42.134Z',14);
INSERT INTO "business_revisions" VALUES('67c7d8fe-6296-43f4-9c3a-64ed31009755','59800d9f-ab8b-4637-8d09-05dec46ce67e','df145c74-6e5a-4372-b633-cab024a4228e','confirmed',1,'{"kind":"sale","version":1,"data":{"material_id":"3c339cdf-f952-4399-bc70-a9541335a732","material_version_id":"f0dc73c8-4246-44d0-90c3-760ddd426515","recipe_version_id":"e2640454-ac7e-4003-9de3-1a2145214398","count":"1","note":"가상 판매 개수 정정"}}','2026-10-24T16:28:42.192Z',15);
INSERT INTO "business_revisions" VALUES('d3e859d5-5fb3-40a8-969a-f5b8456786cc','d0746729-8f85-494f-8f0e-22c8137563b5',NULL,'draft',1,'{"kind":"production","version":1,"data":{"inputs":[{"material_id":"5c7aae45-c1bb-41c0-a205-4ddeaa5cba81","material_version_id":"c3348e3b-1396-4d2a-be07-2b9ecf6785de","quantity":"1.","unit":"kg"}],"outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"","allocation_percent":""}],"note":"가상 미완성 생산"}}','2026-10-24T16:28:42.286Z',16);
INSERT INTO "business_revisions" VALUES('cd919467-794b-486f-bc95-93d2f093a951','21779c4d-d38a-4644-b033-0ed4c086ff40',NULL,'draft',1,'{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":null,"material_version_id":null},"quantity":"1.","unit":"","amount":"","receipt_date":"","note":"가상 미완성 입고","stock_effect":{"kind":"needs_reconciliation","basis_id":null,"included_quantity":""}}}','2026-10-24T16:28:42.349Z',17);
INSERT INTO "business_heads" VALUES('2dbed585-ac74-470a-bd12-0aaeaa0b0e67','fc716d0d-75ac-4730-aad1-acc9ec1ce9f0');
INSERT INTO "business_heads" VALUES('c60fa6fa-f68f-4d3b-b976-aa8f13af603b','c48b8b1c-9638-4d02-8d85-4339aa58f520');
INSERT INTO "business_heads" VALUES('a6dc2d71-9542-4ad2-a850-9fafcff31d0e','ed4e8200-d06e-4d67-a209-322ad7f0bdd6');
INSERT INTO "business_heads" VALUES('0d4d5efb-2e2d-47ca-9fff-351838f2acba','67a21cd0-c4c1-4cd1-b332-69a2796de1f0');
INSERT INTO "business_heads" VALUES('59800d9f-ab8b-4637-8d09-05dec46ce67e','67c7d8fe-6296-43f4-9c3a-64ed31009755');
INSERT INTO "business_heads" VALUES('d0746729-8f85-494f-8f0e-22c8137563b5','d3e859d5-5fb3-40a8-969a-f5b8456786cc');
INSERT INTO "business_heads" VALUES('21779c4d-d38a-4644-b033-0ed4c086ff40','cd919467-794b-486f-bc95-93d2f093a951');
INSERT INTO "business_confirmations" VALUES('c2c07e97-902e-4c7f-af02-063e2183649b','2dbed585-ac74-470a-bd12-0aaeaa0b0e67','fc716d0d-75ac-4730-aad1-acc9ec1ce9f0','4dd3a072-ce75-4bfa-97cb-a2812fa7297c','2026-10-24T16:28:41.608Z',4);
INSERT INTO "business_confirmations" VALUES('45963ced-706c-4f9f-8c94-e898e6ee3b0b','c60fa6fa-f68f-4d3b-b976-aa8f13af603b','c48b8b1c-9638-4d02-8d85-4339aa58f520','23c53180-c7f8-40be-8e87-a74488952588','2026-10-24T16:28:41.715Z',6);
INSERT INTO "business_confirmations" VALUES('26099b3f-5312-441f-b9c2-621f39369223','a6dc2d71-9542-4ad2-a850-9fafcff31d0e','ed4e8200-d06e-4d67-a209-322ad7f0bdd6','f88ce438-fc06-4cf5-9b03-fcc0b81ec54f','2026-10-24T16:28:41.853Z',9);
INSERT INTO "business_confirmations" VALUES('a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf','0d4d5efb-2e2d-47ca-9fff-351838f2acba','67a21cd0-c4c1-4cd1-b332-69a2796de1f0','1fb8ba7b-5772-4d53-b614-3b558359f6cd','2026-10-24T16:28:42.054Z',13);
INSERT INTO "business_confirmations" VALUES('3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e','59800d9f-ab8b-4637-8d09-05dec46ce67e','67c7d8fe-6296-43f4-9c3a-64ed31009755','100a2ebc-9236-44a3-98ea-1acca427c56d','2026-10-24T16:28:42.192Z',15);
INSERT INTO "business_corrections" VALUES('a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf','3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e');
INSERT INTO "purchase_orders" VALUES('e75a6474-ff25-4b00-a067-5a7fc5ed9f9f','c2c07e97-902e-4c7f-af02-063e2183649b','2026-10-24T16:28:41.608Z',4);
INSERT INTO "purchase_order_versions" VALUES('3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f','e75a6474-ff25-4b00-a067-5a7fc5ed9f9f',NULL,'c2c07e97-902e-4c7f-af02-063e2183649b','place','open','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T16:28:41.608Z',4);
INSERT INTO "purchase_order_versions" VALUES('ae5c71ed-65c3-4d80-ac40-51ae2c37c19f','e75a6474-ff25-4b00-a067-5a7fc5ed9f9f','3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f','45963ced-706c-4f9f-8c94-e898e6ee3b0b','receive','received','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',3000000000,'g','3','kg','가상 공급처','2026-10-01',30000000000,'가상 주문','2026-10-24T16:28:41.715Z',6);
INSERT INTO "purchase_order_heads" VALUES('e75a6474-ff25-4b00-a067-5a7fc5ed9f9f','ae5c71ed-65c3-4d80-ac40-51ae2c37c19f');
INSERT INTO "receipts" VALUES('250d2cb6-f87c-4f84-a9cf-22b37ea62f39','45963ced-706c-4f9f-8c94-e898e6ee3b0b','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',3000000000,'g','3','kg',30000000000,'2026-10-01','가상 실제 입고','2026-10-24T16:28:41.715Z',6,'order','e75a6474-ff25-4b00-a067-5a7fc5ed9f9f','3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f');
INSERT INTO "order_receipt_closures" VALUES('e75a6474-ff25-4b00-a067-5a7fc5ed9f9f','250d2cb6-f87c-4f84-a9cf-22b37ea62f39','45963ced-706c-4f9f-8c94-e898e6ee3b0b','3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f','3f8cc8e9-ea73-4c3c-a8dc-82aac5b8399f','ae5c71ed-65c3-4d80-ac40-51ae2c37c19f','3000','0');
INSERT INTO "recipe_versions" VALUES('e2640454-ac7e-4003-9de3-1a2145214398','3c339cdf-f952-4399-bc70-a9541335a732','f0dc73c8-4246-44d0-90c3-760ddd426515',NULL,NULL,3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T16:28:41.887Z',10);
INSERT INTO "recipe_versions" VALUES('8aa2e922-b83a-4e48-a056-aa75ebfd54dd','8c6d323c-0ff3-4b48-a7bb-ebca8616dd95','64ed1b2d-0b6d-4a8e-9010-6794534d7b22',NULL,'e2640454-ac7e-4003-9de3-1a2145214398',3,NULL,NULL,NULL,NULL,1000000000,'가상 조리 설명','섞기','10분','2026-10-24T16:28:41.931Z',11);
INSERT INTO "recipe_lines" VALUES('c1037e32-e60a-46ab-84c3-64b918d44647','e2640454-ac7e-4003-9de3-1a2145214398',0,'2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2',100000000,'g','100.00','g','');
INSERT INTO "recipe_lines" VALUES('47404a02-bf6a-44bc-872a-54061b3d0d2f','e2640454-ac7e-4003-9de3-1a2145214398',1,'5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_lines" VALUES('9bcf88a7-26ee-41f4-87ef-5cd339666203','8aa2e922-b83a-4e48-a056-aa75ebfd54dd',0,'2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2',100000000,'g','100.00','g','');
INSERT INTO "recipe_lines" VALUES('36725a89-6820-4768-b3bf-df8c32b3799c','8aa2e922-b83a-4e48-a056-aa75ebfd54dd',1,'5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',0,'g','0','g','가상 적당량');
INSERT INTO "recipe_heads" VALUES('3c339cdf-f952-4399-bc70-a9541335a732','e2640454-ac7e-4003-9de3-1a2145214398');
INSERT INTO "recipe_heads" VALUES('8c6d323c-0ff3-4b48-a7bb-ebca8616dd95','8aa2e922-b83a-4e48-a056-aa75ebfd54dd');
INSERT INTO "productions" VALUES('6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a','26099b3f-5312-441f-b9c2-621f39369223','가상 전량 생산',30000000000,'2026-10-24T16:28:41.853Z',9);
INSERT INTO "production_effects" VALUES('a05297d7-3d72-4ffa-a453-8a414c6f2c36','6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',3000000000,30000000000,0,0);
INSERT INTO "production_effects" VALUES('a6fdd716-d5a8-40d9-92b3-87d4cd4149cc','6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a','2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2',0,0,2000000000,30000000000);
INSERT INTO "production_inputs" VALUES('6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a',0,'a05297d7-3d72-4ffa-a453-8a414c6f2c36','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',3000000000,'g','3.00','kg',30000000000);
INSERT INTO "production_outputs" VALUES('6bb2e2a1-a54c-4cdd-8a21-44d30db02a4a',0,'a6fdd716-d5a8-40d9-92b3-87d4cd4149cc','2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2',2000000000,'g','2','kg',30000000000,100000000);
INSERT INTO "holding_versions" VALUES('a1154033-8f63-4cec-9216-7d2d48fb4e5c','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de',NULL,'250d2cb6-f87c-4f84-a9cf-22b37ea62f39','250d2cb6-f87c-4f84-a9cf-22b37ea62f39',6,3000000000,30000000000,'2026-10-24T16:28:41.715Z',6,NULL,NULL);
INSERT INTO "holding_versions" VALUES('aea60879-d29d-4bfc-8ec3-5c3fde5a6494','5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','c3348e3b-1396-4d2a-be07-2b9ecf6785de','a1154033-8f63-4cec-9216-7d2d48fb4e5c',NULL,'250d2cb6-f87c-4f84-a9cf-22b37ea62f39',6,0,0,'2026-10-24T16:28:41.853Z',9,'a05297d7-3d72-4ffa-a453-8a414c6f2c36',NULL);
INSERT INTO "holding_versions" VALUES('cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17','2167401e-86fc-4e3a-b7d1-dbde01bf41e1','20705838-8ff0-4bcb-8260-494fabafe7c2',NULL,NULL,NULL,9,2000000000,30000000000,'2026-10-24T16:28:41.853Z',9,'a6fdd716-d5a8-40d9-92b3-87d4cd4149cc','a6fdd716-d5a8-40d9-92b3-87d4cd4149cc');
INSERT INTO "holding_heads" VALUES('5c7aae45-c1bb-41c0-a205-4ddeaa5cba81','aea60879-d29d-4bfc-8ec3-5c3fde5a6494','a1154033-8f63-4cec-9216-7d2d48fb4e5c');
INSERT INTO "holding_heads" VALUES('2167401e-86fc-4e3a-b7d1-dbde01bf41e1','cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17','cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17');
INSERT INTO "sales" VALUES('6443ff31-26b8-4442-a6f2-a760218fdcb7','a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf','3c339cdf-f952-4399-bc70-a9541335a732','f0dc73c8-4246-44d0-90c3-760ddd426515','e2640454-ac7e-4003-9de3-1a2145214398',3,1000000000,500000000,500000000,'{"numerator":"500000000","denominator":"1000000000"}','null');
INSERT INTO "sale_cost_rows" VALUES('6443ff31-26b8-4442-a6f2-a760218fdcb7',0,'c1037e32-e60a-46ab-84c3-64b918d44647','holding_average','cc5cb894-bd00-4bc8-bdb8-99ec6c7d5d17',NULL,30000000000,2000000000,NULL,500000000);
INSERT INTO "sale_cost_rows" VALUES('6443ff31-26b8-4442-a6f2-a760218fdcb7',1,'47404a02-bf6a-44bc-872a-54061b3d0d2f','last_valid','a1154033-8f63-4cec-9216-7d2d48fb4e5c',NULL,30000000000,3000000000,NULL,0);
INSERT INTO "sale_versions" VALUES('61a30394-a9d3-4116-bd7e-b099d5f539ed','6443ff31-26b8-4442-a6f2-a760218fdcb7',NULL,'a3bbf091-c56e-4f9a-b102-9ffc18b7f6cf',3,3000000000,'가상 원판매');
INSERT INTO "sale_versions" VALUES('109b3cf2-0176-41dd-ba5d-a6d165b6551a','6443ff31-26b8-4442-a6f2-a760218fdcb7','61a30394-a9d3-4116-bd7e-b099d5f539ed','3a951ae2-c06d-4bd8-877a-4ebdfa7bbe0e',1,1000000000,'가상 판매 개수 정정');
INSERT INTO "sale_heads" VALUES('6443ff31-26b8-4442-a6f2-a760218fdcb7','109b3cf2-0176-41dd-ba5d-a6d165b6551a');
PRAGMA user_version=9;
