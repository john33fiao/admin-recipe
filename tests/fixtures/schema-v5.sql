-- Frozen synthetic T-006 schema5 state captured through its release API.
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
CREATE TABLE purchase_order_versions(id TEXT PRIMARY KEY NOT NULL,order_id TEXT NOT NULL REFERENCES purchase_orders(id),previous_version_id TEXT,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),action TEXT NOT NULL,status TEXT NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,supplier TEXT,expected_date TEXT,estimated_amount_micros INTEGER CHECK(estimated_amount_micros>=0 AND estimated_amount_micros<=9000000000000000000),note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(order_id,id),FOREIGN KEY(order_id,previous_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),CHECK((action IN('place','revise') AND status='open') OR (action='cancel' AND status='cancelled')));
CREATE TABLE purchase_order_heads(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),current_version_id TEXT NOT NULL,FOREIGN KEY(order_id,current_version_id) REFERENCES purchase_order_versions(order_id,id));
CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
INSERT INTO "materials" VALUES('327f9525-636f-43aa-9862-0267e2e62f97','가상 이전 가격 재료','g','2026-10-24T13:20:43.113Z');
INSERT INTO "snapshot_outbox" VALUES(1,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "1",
  "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
  "recorded_at": "2026-10-24T13:20:43.113Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "1"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
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
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(2,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "2",
  "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
  "recorded_at": "2026-10-24T13:20:43.145Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "2"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      }
    ],
    "business_heads": [
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648"
      }
    ],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(3,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "3",
  "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
  "recorded_at": "2026-10-24T13:20:43.192Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "3"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      }
    ],
    "business_heads": [
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(4,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "4",
  "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
  "recorded_at": "2026-10-24T13:20:43.239Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "4"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      }
    ],
    "business_heads": [
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(5,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "5",
  "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
  "recorded_at": "2026-10-24T13:20:43.286Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "5"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "business_heads": [
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(6,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "6",
  "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
  "recorded_at": "2026-10-24T13:20:43.317Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "6"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      }
    ],
    "business_heads": [
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [],
    "purchase_order_versions": [],
    "purchase_order_heads": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(7,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "7",
  "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
  "recorded_at": "2026-10-24T13:20:43.363Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "7"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      },
      {
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "operation": "business.confirm",
        "payload": "[\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",{\"meta\":{\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"6\"},\"expected_head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.363Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"effective_confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\"},\"confirmation\":{\"id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"revision_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"7\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "7"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      },
      {
        "id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "business_heads": [
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "75629d2e-88f6-45be-b894-71669f361a14",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [
      {
        "id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "first_confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "created_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": null,
        "confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "business_record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "action": "place",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "10000",
        "unit": "g",
        "input_quantity": "10",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "100",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "current_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(8,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "8",
  "request_id": "89909146-44e2-4c40-9e91-2fa8070b4d3c",
  "recorded_at": "2026-10-24T13:20:43.424Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "8"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      },
      {
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "operation": "business.confirm",
        "payload": "[\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",{\"meta\":{\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"6\"},\"expected_head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.363Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"effective_confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\"},\"confirmation\":{\"id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"revision_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"7\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "7"
      },
      {
        "request_id": "89909146-44e2-4c40-9e91-2fa8070b4d3c",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"89909146-44e2-4c40-9e91-2fa8070b4d3c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.424Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.424Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"8\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "8"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      },
      {
        "id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.424Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      },
      {
        "id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "92612e23-fdff-4303-8ec7-286197dce730",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.424Z",
        "seq": "8"
      }
    ],
    "business_heads": [
      {
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "current_revision_id": "92612e23-fdff-4303-8ec7-286197dce730"
      },
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "75629d2e-88f6-45be-b894-71669f361a14",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [
      {
        "id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "first_confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "created_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": null,
        "confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "business_record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "action": "place",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "10000",
        "unit": "g",
        "input_quantity": "10",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "100",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "current_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(9,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "9",
  "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
  "recorded_at": "2026-10-24T13:20:43.470Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "9"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      },
      {
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "operation": "business.confirm",
        "payload": "[\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",{\"meta\":{\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"6\"},\"expected_head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.363Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"effective_confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\"},\"confirmation\":{\"id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"revision_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"7\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "7"
      },
      {
        "request_id": "89909146-44e2-4c40-9e91-2fa8070b4d3c",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"89909146-44e2-4c40-9e91-2fa8070b4d3c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.424Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.424Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"8\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "8"
      },
      {
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "operation": "business.confirm",
        "payload": "[\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",{\"meta\":{\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"8\"},\"expected_head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.470Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"effective_confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"revision_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"9\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "9"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      },
      {
        "id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.424Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      },
      {
        "id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "92612e23-fdff-4303-8ec7-286197dce730",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.424Z",
        "seq": "8"
      },
      {
        "id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": "92612e23-fdff-4303-8ec7-286197dce730",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      }
    ],
    "business_heads": [
      {
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "current_revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b"
      },
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "75629d2e-88f6-45be-b894-71669f361a14",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [
      {
        "id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "first_confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "created_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": null,
        "confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "business_record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "action": "place",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "10000",
        "unit": "g",
        "input_quantity": "10",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "100",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "confirmation_id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "business_record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "action": "revise",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "12000",
        "unit": "g",
        "input_quantity": "12",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "120",
        "note": "가상 주문 수정",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "current_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(10,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "10",
  "request_id": "b089bc81-5946-4c03-bb0b-bcd84e15fcf4",
  "recorded_at": "2026-10-24T13:20:43.518Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "10"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      },
      {
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "operation": "business.confirm",
        "payload": "[\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",{\"meta\":{\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"6\"},\"expected_head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.363Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"effective_confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\"},\"confirmation\":{\"id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"revision_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"7\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "7"
      },
      {
        "request_id": "89909146-44e2-4c40-9e91-2fa8070b4d3c",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"89909146-44e2-4c40-9e91-2fa8070b4d3c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.424Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.424Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"8\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "8"
      },
      {
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "operation": "business.confirm",
        "payload": "[\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",{\"meta\":{\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"8\"},\"expected_head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.470Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"effective_confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"revision_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"9\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "9"
      },
      {
        "request_id": "b089bc81-5946-4c03-bb0b-bcd84e15fcf4",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b089bc81-5946-4c03-bb0b-bcd84e15fcf4\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"9\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"kind\":\"purchase_order\",\"head_id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.518Z\",\"updated_at\":\"2026-10-24T13:20:43.518Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\",\"record_id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.518Z\",\"seq\":\"10\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"10\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "10"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      },
      {
        "id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.424Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.518Z",
        "created_sequence": "10",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      },
      {
        "id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "92612e23-fdff-4303-8ec7-286197dce730",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.424Z",
        "seq": "8"
      },
      {
        "id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": "92612e23-fdff-4303-8ec7-286197dce730",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      },
      {
        "id": "4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0",
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "cancel",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.518Z",
        "seq": "10"
      }
    ],
    "business_heads": [
      {
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "current_revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b"
      },
      {
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "current_revision_id": "4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0"
      },
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "75629d2e-88f6-45be-b894-71669f361a14",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [
      {
        "id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "first_confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "created_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": null,
        "confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "business_record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "action": "place",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "10000",
        "unit": "g",
        "input_quantity": "10",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "100",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "confirmation_id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "business_record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "action": "revise",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "12000",
        "unit": "g",
        "input_quantity": "12",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "120",
        "note": "가상 주문 수정",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "current_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c"
      }
    ]
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(11,'{
  "format_version": 2,
  "schema_version": 5,
  "sequence": "11",
  "request_id": "03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7",
  "recorded_at": "2026-10-24T13:20:43.565Z",
  "state": {
    "materials": [
      {
        "id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T13:20:43.113Z"
      }
    ],
    "material_versions": [
      {
        "id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T13:20:43.113Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "current_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f"
      }
    ],
    "material_prices": [
      {
        "id": "c1ef0765-f470-4c8c-9deb-bbf122165de3",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
      "revision": "11"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "b1081a96-de30-4e71-b5ca-61f6474e2310",
        "operation": "material.create",
        "payload": "{\"request_id\":\"b1081a96-de30-4e71-b5ca-61f6474e2310\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T13:20:43.113Z\",\"updated_at\":\"2026-10-24T13:20:43.113Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"1\"}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "1"
      },
      {
        "request_id": "12a41224-f195-4939-9e92-82b24ec947d3",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"12a41224-f195-4939-9e92-82b24ec947d3\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.145Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.145Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"2\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "2"
      },
      {
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "operation": "business.confirm",
        "payload": "[\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",{\"meta\":{\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"2\"},\"expected_head_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"kind\":\"catalog_price\",\"head_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.145Z\",\"updated_at\":\"2026-10-24T13:20:43.192Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"previous_revision_id\":\"2edae333-4228-4fe5-893e-ca14d2902648\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"record_id\":\"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4\",\"revision_id\":\"51da3f5e-925c-4da8-96e2-cb81b558162a\",\"request_id\":\"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2\",\"recorded_at\":\"2026-10-24T13:20:43.192Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"c1ef0765-f470-4c8c-9deb-bbf122165de3\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"3\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "3"
      },
      {
        "request_id": "d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.239Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":null,\"effective_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.239Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"4\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "4"
      },
      {
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "operation": "business.confirm",
        "payload": "[\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",{\"meta\":{\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"4\"},\"expected_head_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"kind\":\"catalog_price\",\"head_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.239Z\",\"updated_at\":\"2026-10-24T13:20:43.286Z\",\"corrects_confirmation_id\":\"faa39669-aaa0-4ee9-bfa5-fb0a94977768\",\"confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"effective_confirmation_id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"previous_revision_id\":\"8e22275e-911a-4d5c-9091-7f3d246b7bde\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"e5c4dfe0-3d60-4556-bdfa-c13e052628a0\",\"record_id\":\"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40\",\"revision_id\":\"c1d607dc-b4e1-4379-b243-5ce57746ef6d\",\"request_id\":\"f5ce170b-3cb6-4003-b063-c5023eae67eb\",\"recorded_at\":\"2026-10-24T13:20:43.286Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86c1f8ef-47f3-480e-bd4e-ea0213dc0239\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"5\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "5"
      },
      {
        "request_id": "4fed0154-26e6-4b5e-9b77-c2cfd73f12e7",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"5\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.317Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.317Z\",\"seq\":\"6\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"6\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "6"
      },
      {
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "operation": "business.confirm",
        "payload": "[\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",{\"meta\":{\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"6\"},\"expected_head_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"kind\":\"purchase_order\",\"head_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.317Z\",\"updated_at\":\"2026-10-24T13:20:43.363Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"effective_confirmation_id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"previous_revision_id\":\"b6c6ee45-09ca-453b-8c80-c120e1b12230\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"place\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"10\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"100\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\"},\"confirmation\":{\"id\":\"75629d2e-88f6-45be-b894-71669f361a14\",\"record_id\":\"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9\",\"revision_id\":\"7de3b7ef-5cd9-42ef-b27a-a358d31968ec\",\"request_id\":\"7e065582-09a2-4938-872a-15e06b80a13c\",\"recorded_at\":\"2026-10-24T13:20:43.363Z\",\"seq\":\"7\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"place\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"7\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "7"
      },
      {
        "request_id": "89909146-44e2-4c40-9e91-2fa8070b4d3c",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"89909146-44e2-4c40-9e91-2fa8070b4d3c\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"7\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.424Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.424Z\",\"seq\":\"8\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"8\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "8"
      },
      {
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "operation": "business.confirm",
        "payload": "[\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",{\"meta\":{\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"8\"},\"expected_head_id\":\"92612e23-fdff-4303-8ec7-286197dce730\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"kind\":\"purchase_order\",\"head_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.424Z\",\"updated_at\":\"2026-10-24T13:20:43.470Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"effective_confirmation_id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"previous_revision_id\":\"92612e23-fdff-4303-8ec7-286197dce730\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\",\"quantity\":\"12\",\"unit\":\"kg\",\"supplier\":\"가상 이전 거래처\",\"expected_date\":\"2026-10-01\",\"estimated_amount\":\"120\",\"note\":\"가상 주문 수정\"}},\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\"},\"confirmation\":{\"id\":\"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad\",\"record_id\":\"16a8f6f2-915a-42fe-9a26-fcf65550037a\",\"revision_id\":\"3c632c12-3275-4a52-8453-5de3fcc45b4b\",\"request_id\":\"17c93c71-53fe-4bb3-830b-0727eb3f601a\",\"recorded_at\":\"2026-10-24T13:20:43.470Z\",\"seq\":\"9\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"revise\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"9\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "9"
      },
      {
        "request_id": "b089bc81-5946-4c03-bb0b-bcd84e15fcf4",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"b089bc81-5946-4c03-bb0b-bcd84e15fcf4\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"9\"},\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"kind\":\"purchase_order\",\"head_id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\",\"status\":\"draft\",\"created_at\":\"2026-10-24T13:20:43.518Z\",\"updated_at\":\"2026-10-24T13:20:43.518Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\",\"record_id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.518Z\",\"seq\":\"10\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"10\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "10"
      },
      {
        "request_id": "03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7",
        "operation": "business.confirm",
        "payload": "[\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",{\"meta\":{\"request_id\":\"03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7\",\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"expected_revision\":\"10\"},\"expected_head_id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"kind\":\"purchase_order\",\"head_id\":\"be88f9f1-cdd4-4b97-8c25-1dacedfc9492\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T13:20:43.518Z\",\"updated_at\":\"2026-10-24T13:20:43.565Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b\",\"effective_confirmation_id\":\"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"be88f9f1-cdd4-4b97-8c25-1dacedfc9492\",\"record_id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"previous_revision_id\":\"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"purchase_order\",\"version\":1,\"data\":{\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"expected_order_version_id\":\"a15fe39b-516b-4cce-accb-b1ce0ca6b31c\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T13:20:43.565Z\",\"seq\":\"11\"},\"confirmation\":{\"id\":\"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b\",\"record_id\":\"4b4ecd26-b462-4169-8d6b-d0fc26b8e948\",\"revision_id\":\"be88f9f1-cdd4-4b97-8c25-1dacedfc9492\",\"request_id\":\"03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7\",\"recorded_at\":\"2026-10-24T13:20:43.565Z\",\"seq\":\"11\",\"result\":{\"kind\":\"purchase_order\",\"action\":\"cancel\",\"order_id\":\"994210b2-6284-4802-a03f-1c9d013944fa\",\"order_version_id\":\"2ce01cb4-8cc5-458e-a79b-1023ac042caf\",\"material_id\":\"327f9525-636f-43aa-9862-0267e2e62f97\",\"material_version_id\":\"2f30a0d3-b0ec-45d6-8aea-af755810e56f\"}},\"issues\":[]},\"state\":{\"epoch\":\"f887a4d9-5e37-4084-b33f-3cd77054f30f\",\"revision\":\"11\"}}",
        "epoch": "f887a4d9-5e37-4084-b33f-3cd77054f30f",
        "sequence": "11"
      }
    ],
    "business_records": [
      {
        "id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.145Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "kind": "catalog_price",
        "created_at": "2026-10-24T13:20:43.239Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768"
      },
      {
        "id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.317Z",
        "created_sequence": "6",
        "corrects_confirmation_id": null
      },
      {
        "id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.424Z",
        "created_sequence": "8",
        "corrects_confirmation_id": null
      },
      {
        "id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "kind": "purchase_order",
        "created_at": "2026-10-24T13:20:43.518Z",
        "created_sequence": "10",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.145Z",
        "seq": "2"
      },
      {
        "id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "previous_revision_id": "2edae333-4228-4fe5-893e-ca14d2902648",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.239Z",
        "seq": "4"
      },
      {
        "id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "previous_revision_id": "8e22275e-911a-4d5c-9091-7f3d246b7bde",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.317Z",
        "seq": "6"
      },
      {
        "id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "previous_revision_id": "b6c6ee45-09ca-453b-8c80-c120e1b12230",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "place",
            "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "10",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "100",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "92612e23-fdff-4303-8ec7-286197dce730",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.424Z",
        "seq": "8"
      },
      {
        "id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "previous_revision_id": "92612e23-fdff-4303-8ec7-286197dce730",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "revise",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
            "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
            "quantity": "12",
            "unit": "kg",
            "supplier": "가상 이전 거래처",
            "expected_date": "2026-10-01",
            "estimated_amount": "120",
            "note": "가상 주문 수정"
          }
        },
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      },
      {
        "id": "4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0",
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "cancel",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.518Z",
        "seq": "10"
      },
      {
        "id": "be88f9f1-cdd4-4b97-8c25-1dacedfc9492",
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "previous_revision_id": "4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0",
        "status": "confirmed",
        "payload": {
          "kind": "purchase_order",
          "version": 1,
          "data": {
            "action": "cancel",
            "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
            "expected_order_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T13:20:43.565Z",
        "seq": "11"
      }
    ],
    "business_heads": [
      {
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "current_revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b"
      },
      {
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "current_revision_id": "be88f9f1-cdd4-4b97-8c25-1dacedfc9492"
      },
      {
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "current_revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec"
      },
      {
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "current_revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d"
      },
      {
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "current_revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a"
      }
    ],
    "business_confirmations": [
      {
        "id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "record_id": "fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",
        "revision_id": "51da3f5e-925c-4da8-96e2-cb81b558162a",
        "request_id": "800f9949-0fc2-4a79-b7da-7e1da5f7bfb2",
        "recorded_at": "2026-10-24T13:20:43.192Z",
        "seq": "3"
      },
      {
        "id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "record_id": "ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",
        "revision_id": "c1d607dc-b4e1-4379-b243-5ce57746ef6d",
        "request_id": "f5ce170b-3cb6-4003-b063-c5023eae67eb",
        "recorded_at": "2026-10-24T13:20:43.286Z",
        "seq": "5"
      },
      {
        "id": "75629d2e-88f6-45be-b894-71669f361a14",
        "record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "revision_id": "7de3b7ef-5cd9-42ef-b27a-a358d31968ec",
        "request_id": "7e065582-09a2-4938-872a-15e06b80a13c",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "revision_id": "3c632c12-3275-4a52-8453-5de3fcc45b4b",
        "request_id": "17c93c71-53fe-4bb3-830b-0727eb3f601a",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      },
      {
        "id": "8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b",
        "record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "revision_id": "be88f9f1-cdd4-4b97-8c25-1dacedfc9492",
        "request_id": "03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7",
        "recorded_at": "2026-10-24T13:20:43.565Z",
        "seq": "11"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0",
        "price_id": "86c1f8ef-47f3-480e-bd4e-ea0213dc0239"
      },
      {
        "confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "price_id": "c1ef0765-f470-4c8c-9deb-bbf122165de3"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "faa39669-aaa0-4ee9-bfa5-fb0a94977768",
        "replacement_confirmation_id": "e5c4dfe0-3d60-4556-bdfa-c13e052628a0"
      }
    ],
    "purchase_orders": [
      {
        "id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "first_confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "created_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      }
    ],
    "purchase_order_versions": [
      {
        "id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": null,
        "confirmation_id": "75629d2e-88f6-45be-b894-71669f361a14",
        "business_record_id": "afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",
        "action": "place",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "10000",
        "unit": "g",
        "input_quantity": "10",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "100",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.363Z",
        "seq": "7"
      },
      {
        "id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": "fb9ec870-92a5-4ba3-98fe-c12c930ccbd7",
        "confirmation_id": "7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad",
        "business_record_id": "16a8f6f2-915a-42fe-9a26-fcf65550037a",
        "action": "revise",
        "status": "open",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "12000",
        "unit": "g",
        "input_quantity": "12",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "120",
        "note": "가상 주문 수정",
        "recorded_at": "2026-10-24T13:20:43.470Z",
        "seq": "9"
      },
      {
        "id": "2ce01cb4-8cc5-458e-a79b-1023ac042caf",
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "previous_version_id": "a15fe39b-516b-4cce-accb-b1ce0ca6b31c",
        "confirmation_id": "8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b",
        "business_record_id": "4b4ecd26-b462-4169-8d6b-d0fc26b8e948",
        "action": "cancel",
        "status": "cancelled",
        "material_id": "327f9525-636f-43aa-9862-0267e2e62f97",
        "material_version_id": "2f30a0d3-b0ec-45d6-8aea-af755810e56f",
        "material_name": "가상 이전 가격 재료",
        "quantity": "12000",
        "unit": "g",
        "input_quantity": "12",
        "input_unit": "kg",
        "supplier": "가상 이전 거래처",
        "expected_date": "2026-10-01",
        "estimated_amount": "120",
        "note": "",
        "recorded_at": "2026-10-24T13:20:43.565Z",
        "seq": "11"
      }
    ],
    "purchase_order_heads": [
      {
        "order_id": "994210b2-6284-4802-a03f-1c9d013944fa",
        "current_version_id": "2ce01cb4-8cc5-458e-a79b-1023ac042caf"
      }
    ]
  }
}
',1);
INSERT INTO "material_versions" VALUES('2f30a0d3-b0ec-45d6-8aea-af755810e56f','327f9525-636f-43aa-9862-0267e2e62f97','가상 이전 가격 재료','g','g','mass','raw','형식 보존 검증용',0,'2026-10-24T13:20:43.113Z',1,'catalog');
INSERT INTO "material_heads" VALUES('327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f');
INSERT INTO "material_prices" VALUES('c1ef0765-f470-4c8c-9deb-bbf122165de3','327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f',1000000000,10000000,'g','1','kg','2026-10-24T13:20:43.192Z',3);
INSERT INTO "material_prices" VALUES('86c1f8ef-47f3-480e-bd4e-ea0213dc0239','327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f',1000000000,30000000,'g','1','kg','2026-10-24T13:20:43.286Z',5);
INSERT INTO "mutation_requests" VALUES('b1081a96-de30-4e71-b5ca-61f6474e2310','material.create','{"request_id":"b1081a96-de30-4e71-b5ca-61f6474e2310","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"0","name":"가상 이전 가격 재료","unit":"g","kind":"raw","description":"형식 보존 검증용"}','{"id":"327f9525-636f-43aa-9862-0267e2e62f97","version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","name":"가상 이전 가격 재료","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"형식 보존 검증용","archived":false,"created_at":"2026-10-24T13:20:43.113Z","updated_at":"2026-10-24T13:20:43.113Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"1"}','f887a4d9-5e37-4084-b33f-3cd77054f30f',1);
INSERT INTO "mutation_requests" VALUES('12a41224-f195-4939-9e92-82b24ec947d3','business.create','{"meta":{"request_id":"12a41224-f195-4939-9e92-82b24ec947d3","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"1"},"payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"10","note":""}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4","kind":"catalog_price","head_id":"2edae333-4228-4fe5-893e-ca14d2902648","status":"draft","created_at":"2026-10-24T13:20:43.145Z","updated_at":"2026-10-24T13:20:43.145Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"2edae333-4228-4fe5-893e-ca14d2902648","record_id":"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4","previous_revision_id":null,"status":"draft","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"10","note":""}},"recorded_at":"2026-10-24T13:20:43.145Z","seq":"2"},"confirmation":null,"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"2"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',2);
INSERT INTO "mutation_requests" VALUES('800f9949-0fc2-4a79-b7da-7e1da5f7bfb2','business.confirm','["fbdab3ea-4aef-4b84-97ca-d4dc937f28c4",{"meta":{"request_id":"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"2"},"expected_head_id":"2edae333-4228-4fe5-893e-ca14d2902648"}]','{"result":{"item":{"id":"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4","kind":"catalog_price","head_id":"51da3f5e-925c-4da8-96e2-cb81b558162a","status":"confirmed","created_at":"2026-10-24T13:20:43.145Z","updated_at":"2026-10-24T13:20:43.192Z","corrects_confirmation_id":null,"confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","effective_confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"51da3f5e-925c-4da8-96e2-cb81b558162a","record_id":"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4","previous_revision_id":"2edae333-4228-4fe5-893e-ca14d2902648","status":"confirmed","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"10","note":""}},"recorded_at":"2026-10-24T13:20:43.192Z","seq":"3"},"confirmation":{"id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","record_id":"fbdab3ea-4aef-4b84-97ca-d4dc937f28c4","revision_id":"51da3f5e-925c-4da8-96e2-cb81b558162a","request_id":"800f9949-0fc2-4a79-b7da-7e1da5f7bfb2","recorded_at":"2026-10-24T13:20:43.192Z","seq":"3","result":{"kind":"catalog_price","price_id":"c1ef0765-f470-4c8c-9deb-bbf122165de3","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f"}},"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"3"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',3);
INSERT INTO "mutation_requests" VALUES('d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51','business.create','{"meta":{"request_id":"d94f7b99-8578-4ed7-a9ac-5fdabfb7ef51","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"3"},"payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"corrects_confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768"}','{"result":{"item":{"id":"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40","kind":"catalog_price","head_id":"8e22275e-911a-4d5c-9091-7f3d246b7bde","status":"draft","created_at":"2026-10-24T13:20:43.239Z","updated_at":"2026-10-24T13:20:43.239Z","corrects_confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","confirmation_id":null,"effective_confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"8e22275e-911a-4d5c-9091-7f3d246b7bde","record_id":"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40","previous_revision_id":null,"status":"draft","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"recorded_at":"2026-10-24T13:20:43.239Z","seq":"4"},"confirmation":null,"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"4"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',4);
INSERT INTO "mutation_requests" VALUES('f5ce170b-3cb6-4003-b063-c5023eae67eb','business.confirm','["ca781e66-2aa8-49f8-9c2a-d0f1debdeb40",{"meta":{"request_id":"f5ce170b-3cb6-4003-b063-c5023eae67eb","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"4"},"expected_head_id":"8e22275e-911a-4d5c-9091-7f3d246b7bde"}]','{"result":{"item":{"id":"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40","kind":"catalog_price","head_id":"c1d607dc-b4e1-4379-b243-5ce57746ef6d","status":"confirmed","created_at":"2026-10-24T13:20:43.239Z","updated_at":"2026-10-24T13:20:43.286Z","corrects_confirmation_id":"faa39669-aaa0-4ee9-bfa5-fb0a94977768","confirmation_id":"e5c4dfe0-3d60-4556-bdfa-c13e052628a0","effective_confirmation_id":"e5c4dfe0-3d60-4556-bdfa-c13e052628a0","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"c1d607dc-b4e1-4379-b243-5ce57746ef6d","record_id":"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40","previous_revision_id":"8e22275e-911a-4d5c-9091-7f3d246b7bde","status":"confirmed","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"recorded_at":"2026-10-24T13:20:43.286Z","seq":"5"},"confirmation":{"id":"e5c4dfe0-3d60-4556-bdfa-c13e052628a0","record_id":"ca781e66-2aa8-49f8-9c2a-d0f1debdeb40","revision_id":"c1d607dc-b4e1-4379-b243-5ce57746ef6d","request_id":"f5ce170b-3cb6-4003-b063-c5023eae67eb","recorded_at":"2026-10-24T13:20:43.286Z","seq":"5","result":{"kind":"catalog_price","price_id":"86c1f8ef-47f3-480e-bd4e-ea0213dc0239","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f"}},"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"5"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',5);
INSERT INTO "mutation_requests" VALUES('4fed0154-26e6-4b5e-9b77-c2cfd73f12e7','business.create','{"meta":{"request_id":"4fed0154-26e6-4b5e-9b77-c2cfd73f12e7","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"5"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"10","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"100","note":""}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9","kind":"purchase_order","head_id":"b6c6ee45-09ca-453b-8c80-c120e1b12230","status":"draft","created_at":"2026-10-24T13:20:43.317Z","updated_at":"2026-10-24T13:20:43.317Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"b6c6ee45-09ca-453b-8c80-c120e1b12230","record_id":"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"10","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"100","note":""}},"recorded_at":"2026-10-24T13:20:43.317Z","seq":"6"},"confirmation":null,"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"6"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',6);
INSERT INTO "mutation_requests" VALUES('7e065582-09a2-4938-872a-15e06b80a13c','business.confirm','["afd98a4a-d057-420e-aaf6-ffbc0a29fbf9",{"meta":{"request_id":"7e065582-09a2-4938-872a-15e06b80a13c","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"6"},"expected_head_id":"b6c6ee45-09ca-453b-8c80-c120e1b12230"}]','{"result":{"item":{"id":"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9","kind":"purchase_order","head_id":"7de3b7ef-5cd9-42ef-b27a-a358d31968ec","status":"confirmed","created_at":"2026-10-24T13:20:43.317Z","updated_at":"2026-10-24T13:20:43.363Z","corrects_confirmation_id":null,"confirmation_id":"75629d2e-88f6-45be-b894-71669f361a14","effective_confirmation_id":"75629d2e-88f6-45be-b894-71669f361a14","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"7de3b7ef-5cd9-42ef-b27a-a358d31968ec","record_id":"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9","previous_revision_id":"b6c6ee45-09ca-453b-8c80-c120e1b12230","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"10","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"100","note":""}},"recorded_at":"2026-10-24T13:20:43.363Z","seq":"7"},"confirmation":{"id":"75629d2e-88f6-45be-b894-71669f361a14","record_id":"afd98a4a-d057-420e-aaf6-ffbc0a29fbf9","revision_id":"7de3b7ef-5cd9-42ef-b27a-a358d31968ec","request_id":"7e065582-09a2-4938-872a-15e06b80a13c","recorded_at":"2026-10-24T13:20:43.363Z","seq":"7","result":{"kind":"purchase_order","action":"place","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f"}},"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"7"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',7);
INSERT INTO "mutation_requests" VALUES('89909146-44e2-4c40-9e91-2fa8070b4d3c','business.create','{"meta":{"request_id":"89909146-44e2-4c40-9e91-2fa8070b4d3c","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"7"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"12","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"120","note":"가상 주문 수정"}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"16a8f6f2-915a-42fe-9a26-fcf65550037a","kind":"purchase_order","head_id":"92612e23-fdff-4303-8ec7-286197dce730","status":"draft","created_at":"2026-10-24T13:20:43.424Z","updated_at":"2026-10-24T13:20:43.424Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"92612e23-fdff-4303-8ec7-286197dce730","record_id":"16a8f6f2-915a-42fe-9a26-fcf65550037a","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"12","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"120","note":"가상 주문 수정"}},"recorded_at":"2026-10-24T13:20:43.424Z","seq":"8"},"confirmation":null,"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"8"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',8);
INSERT INTO "mutation_requests" VALUES('17c93c71-53fe-4bb3-830b-0727eb3f601a','business.confirm','["16a8f6f2-915a-42fe-9a26-fcf65550037a",{"meta":{"request_id":"17c93c71-53fe-4bb3-830b-0727eb3f601a","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"8"},"expected_head_id":"92612e23-fdff-4303-8ec7-286197dce730"}]','{"result":{"item":{"id":"16a8f6f2-915a-42fe-9a26-fcf65550037a","kind":"purchase_order","head_id":"3c632c12-3275-4a52-8453-5de3fcc45b4b","status":"confirmed","created_at":"2026-10-24T13:20:43.424Z","updated_at":"2026-10-24T13:20:43.470Z","corrects_confirmation_id":null,"confirmation_id":"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad","effective_confirmation_id":"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"3c632c12-3275-4a52-8453-5de3fcc45b4b","record_id":"16a8f6f2-915a-42fe-9a26-fcf65550037a","previous_revision_id":"92612e23-fdff-4303-8ec7-286197dce730","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"12","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"120","note":"가상 주문 수정"}},"recorded_at":"2026-10-24T13:20:43.470Z","seq":"9"},"confirmation":{"id":"7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad","record_id":"16a8f6f2-915a-42fe-9a26-fcf65550037a","revision_id":"3c632c12-3275-4a52-8453-5de3fcc45b4b","request_id":"17c93c71-53fe-4bb3-830b-0727eb3f601a","recorded_at":"2026-10-24T13:20:43.470Z","seq":"9","result":{"kind":"purchase_order","action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f"}},"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"9"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',9);
INSERT INTO "mutation_requests" VALUES('b089bc81-5946-4c03-bb0b-bcd84e15fcf4','business.create','{"meta":{"request_id":"b089bc81-5946-4c03-bb0b-bcd84e15fcf4","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"9"},"payload":{"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","note":""}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"4b4ecd26-b462-4169-8d6b-d0fc26b8e948","kind":"purchase_order","head_id":"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0","status":"draft","created_at":"2026-10-24T13:20:43.518Z","updated_at":"2026-10-24T13:20:43.518Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0","record_id":"4b4ecd26-b462-4169-8d6b-d0fc26b8e948","previous_revision_id":null,"status":"draft","payload":{"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","note":""}},"recorded_at":"2026-10-24T13:20:43.518Z","seq":"10"},"confirmation":null,"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"10"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',10);
INSERT INTO "mutation_requests" VALUES('03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7','business.confirm','["4b4ecd26-b462-4169-8d6b-d0fc26b8e948",{"meta":{"request_id":"03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7","epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","expected_revision":"10"},"expected_head_id":"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0"}]','{"result":{"item":{"id":"4b4ecd26-b462-4169-8d6b-d0fc26b8e948","kind":"purchase_order","head_id":"be88f9f1-cdd4-4b97-8c25-1dacedfc9492","status":"confirmed","created_at":"2026-10-24T13:20:43.518Z","updated_at":"2026-10-24T13:20:43.565Z","corrects_confirmation_id":null,"confirmation_id":"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b","effective_confirmation_id":"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"be88f9f1-cdd4-4b97-8c25-1dacedfc9492","record_id":"4b4ecd26-b462-4169-8d6b-d0fc26b8e948","previous_revision_id":"4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0","status":"confirmed","payload":{"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","note":""}},"recorded_at":"2026-10-24T13:20:43.565Z","seq":"11"},"confirmation":{"id":"8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b","record_id":"4b4ecd26-b462-4169-8d6b-d0fc26b8e948","revision_id":"be88f9f1-cdd4-4b97-8c25-1dacedfc9492","request_id":"03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7","recorded_at":"2026-10-24T13:20:43.565Z","seq":"11","result":{"kind":"purchase_order","action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","order_version_id":"2ce01cb4-8cc5-458e-a79b-1023ac042caf","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f"}},"issues":[]},"state":{"epoch":"f887a4d9-5e37-4084-b33f-3cd77054f30f","revision":"11"}}','f887a4d9-5e37-4084-b33f-3cd77054f30f',11);
INSERT INTO "catalog_state" VALUES(1,'f887a4d9-5e37-4084-b33f-3cd77054f30f',11);
INSERT INTO "business_records" VALUES('fbdab3ea-4aef-4b84-97ca-d4dc937f28c4','catalog_price','2026-10-24T13:20:43.145Z',2,NULL);
INSERT INTO "business_records" VALUES('ca781e66-2aa8-49f8-9c2a-d0f1debdeb40','catalog_price','2026-10-24T13:20:43.239Z',4,'faa39669-aaa0-4ee9-bfa5-fb0a94977768');
INSERT INTO "business_records" VALUES('afd98a4a-d057-420e-aaf6-ffbc0a29fbf9','purchase_order','2026-10-24T13:20:43.317Z',6,NULL);
INSERT INTO "business_records" VALUES('16a8f6f2-915a-42fe-9a26-fcf65550037a','purchase_order','2026-10-24T13:20:43.424Z',8,NULL);
INSERT INTO "business_records" VALUES('4b4ecd26-b462-4169-8d6b-d0fc26b8e948','purchase_order','2026-10-24T13:20:43.518Z',10,NULL);
INSERT INTO "business_revisions" VALUES('2edae333-4228-4fe5-893e-ca14d2902648','fbdab3ea-4aef-4b84-97ca-d4dc937f28c4',NULL,'draft',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"10","note":""}}','2026-10-24T13:20:43.145Z',2);
INSERT INTO "business_revisions" VALUES('51da3f5e-925c-4da8-96e2-cb81b558162a','fbdab3ea-4aef-4b84-97ca-d4dc937f28c4','2edae333-4228-4fe5-893e-ca14d2902648','confirmed',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"10","note":""}}','2026-10-24T13:20:43.192Z',3);
INSERT INTO "business_revisions" VALUES('8e22275e-911a-4d5c-9091-7f3d246b7bde','ca781e66-2aa8-49f8-9c2a-d0f1debdeb40',NULL,'draft',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}}','2026-10-24T13:20:43.239Z',4);
INSERT INTO "business_revisions" VALUES('c1d607dc-b4e1-4379-b243-5ce57746ef6d','ca781e66-2aa8-49f8-9c2a-d0f1debdeb40','8e22275e-911a-4d5c-9091-7f3d246b7bde','confirmed',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}}','2026-10-24T13:20:43.286Z',5);
INSERT INTO "business_revisions" VALUES('b6c6ee45-09ca-453b-8c80-c120e1b12230','afd98a4a-d057-420e-aaf6-ffbc0a29fbf9',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"10","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"100","note":""}}','2026-10-24T13:20:43.317Z',6);
INSERT INTO "business_revisions" VALUES('7de3b7ef-5cd9-42ef-b27a-a358d31968ec','afd98a4a-d057-420e-aaf6-ffbc0a29fbf9','b6c6ee45-09ca-453b-8c80-c120e1b12230','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"place","material_id":"327f9525-636f-43aa-9862-0267e2e62f97","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"10","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"100","note":""}}','2026-10-24T13:20:43.363Z',7);
INSERT INTO "business_revisions" VALUES('92612e23-fdff-4303-8ec7-286197dce730','16a8f6f2-915a-42fe-9a26-fcf65550037a',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"12","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"120","note":"가상 주문 수정"}}','2026-10-24T13:20:43.424Z',8);
INSERT INTO "business_revisions" VALUES('3c632c12-3275-4a52-8453-5de3fcc45b4b','16a8f6f2-915a-42fe-9a26-fcf65550037a','92612e23-fdff-4303-8ec7-286197dce730','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"fb9ec870-92a5-4ba3-98fe-c12c930ccbd7","material_version_id":"2f30a0d3-b0ec-45d6-8aea-af755810e56f","quantity":"12","unit":"kg","supplier":"가상 이전 거래처","expected_date":"2026-10-01","estimated_amount":"120","note":"가상 주문 수정"}}','2026-10-24T13:20:43.470Z',9);
INSERT INTO "business_revisions" VALUES('4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0','4b4ecd26-b462-4169-8d6b-d0fc26b8e948',NULL,'draft',1,'{"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","note":""}}','2026-10-24T13:20:43.518Z',10);
INSERT INTO "business_revisions" VALUES('be88f9f1-cdd4-4b97-8c25-1dacedfc9492','4b4ecd26-b462-4169-8d6b-d0fc26b8e948','4cd3c3a2-6818-4c6b-9a9f-b2150bd1e8c0','confirmed',1,'{"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":"994210b2-6284-4802-a03f-1c9d013944fa","expected_order_version_id":"a15fe39b-516b-4cce-accb-b1ce0ca6b31c","note":""}}','2026-10-24T13:20:43.565Z',11);
INSERT INTO "business_heads" VALUES('fbdab3ea-4aef-4b84-97ca-d4dc937f28c4','51da3f5e-925c-4da8-96e2-cb81b558162a');
INSERT INTO "business_heads" VALUES('ca781e66-2aa8-49f8-9c2a-d0f1debdeb40','c1d607dc-b4e1-4379-b243-5ce57746ef6d');
INSERT INTO "business_heads" VALUES('afd98a4a-d057-420e-aaf6-ffbc0a29fbf9','7de3b7ef-5cd9-42ef-b27a-a358d31968ec');
INSERT INTO "business_heads" VALUES('16a8f6f2-915a-42fe-9a26-fcf65550037a','3c632c12-3275-4a52-8453-5de3fcc45b4b');
INSERT INTO "business_heads" VALUES('4b4ecd26-b462-4169-8d6b-d0fc26b8e948','be88f9f1-cdd4-4b97-8c25-1dacedfc9492');
INSERT INTO "business_confirmations" VALUES('faa39669-aaa0-4ee9-bfa5-fb0a94977768','fbdab3ea-4aef-4b84-97ca-d4dc937f28c4','51da3f5e-925c-4da8-96e2-cb81b558162a','800f9949-0fc2-4a79-b7da-7e1da5f7bfb2','2026-10-24T13:20:43.192Z',3);
INSERT INTO "business_confirmations" VALUES('e5c4dfe0-3d60-4556-bdfa-c13e052628a0','ca781e66-2aa8-49f8-9c2a-d0f1debdeb40','c1d607dc-b4e1-4379-b243-5ce57746ef6d','f5ce170b-3cb6-4003-b063-c5023eae67eb','2026-10-24T13:20:43.286Z',5);
INSERT INTO "business_confirmations" VALUES('75629d2e-88f6-45be-b894-71669f361a14','afd98a4a-d057-420e-aaf6-ffbc0a29fbf9','7de3b7ef-5cd9-42ef-b27a-a358d31968ec','7e065582-09a2-4938-872a-15e06b80a13c','2026-10-24T13:20:43.363Z',7);
INSERT INTO "business_confirmations" VALUES('7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad','16a8f6f2-915a-42fe-9a26-fcf65550037a','3c632c12-3275-4a52-8453-5de3fcc45b4b','17c93c71-53fe-4bb3-830b-0727eb3f601a','2026-10-24T13:20:43.470Z',9);
INSERT INTO "business_confirmations" VALUES('8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b','4b4ecd26-b462-4169-8d6b-d0fc26b8e948','be88f9f1-cdd4-4b97-8c25-1dacedfc9492','03fcabc2-96e3-43ee-88c6-bf6c27f3ddf7','2026-10-24T13:20:43.565Z',11);
INSERT INTO "business_catalog_price_results" VALUES('faa39669-aaa0-4ee9-bfa5-fb0a94977768','c1ef0765-f470-4c8c-9deb-bbf122165de3');
INSERT INTO "business_catalog_price_results" VALUES('e5c4dfe0-3d60-4556-bdfa-c13e052628a0','86c1f8ef-47f3-480e-bd4e-ea0213dc0239');
INSERT INTO "business_corrections" VALUES('faa39669-aaa0-4ee9-bfa5-fb0a94977768','e5c4dfe0-3d60-4556-bdfa-c13e052628a0');
INSERT INTO "purchase_orders" VALUES('994210b2-6284-4802-a03f-1c9d013944fa','75629d2e-88f6-45be-b894-71669f361a14','2026-10-24T13:20:43.363Z',7);
INSERT INTO "purchase_order_versions" VALUES('fb9ec870-92a5-4ba3-98fe-c12c930ccbd7','994210b2-6284-4802-a03f-1c9d013944fa',NULL,'75629d2e-88f6-45be-b894-71669f361a14','place','open','327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f',10000000000,'g','10','kg','가상 이전 거래처','2026-10-01',100000000,'','2026-10-24T13:20:43.363Z',7);
INSERT INTO "purchase_order_versions" VALUES('a15fe39b-516b-4cce-accb-b1ce0ca6b31c','994210b2-6284-4802-a03f-1c9d013944fa','fb9ec870-92a5-4ba3-98fe-c12c930ccbd7','7a70fc92-c7d8-4ce5-ac88-5cdc84b88fad','revise','open','327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f',12000000000,'g','12','kg','가상 이전 거래처','2026-10-01',120000000,'가상 주문 수정','2026-10-24T13:20:43.470Z',9);
INSERT INTO "purchase_order_versions" VALUES('2ce01cb4-8cc5-458e-a79b-1023ac042caf','994210b2-6284-4802-a03f-1c9d013944fa','a15fe39b-516b-4cce-accb-b1ce0ca6b31c','8adb8639-2dd9-4d7c-9d9c-dd3cd67e2d4b','cancel','cancelled','327f9525-636f-43aa-9862-0267e2e62f97','2f30a0d3-b0ec-45d6-8aea-af755810e56f',12000000000,'g','12','kg','가상 이전 거래처','2026-10-01',120000000,'','2026-10-24T13:20:43.565Z',11);
INSERT INTO "purchase_order_heads" VALUES('994210b2-6284-4802-a03f-1c9d013944fa','2ce01cb4-8cc5-458e-a79b-1023ac042caf');
PRAGMA user_version=5;
