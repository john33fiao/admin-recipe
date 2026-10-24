-- Frozen synthetic T-005 schema4 state captured through its release API.
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
CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
CREATE INDEX material_prices_material ON material_prices(material_id,sequence);
CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
INSERT INTO "materials" VALUES('2589dd8e-5026-4f97-897c-ae046c15a4b2','가상 이전 가격 재료','g','2026-10-24T12:48:36.946Z');
INSERT INTO "snapshot_outbox" VALUES(1,'{
  "format_version": 2,
  "schema_version": 4,
  "sequence": "1",
  "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
  "recorded_at": "2026-10-24T12:48:36.946Z",
  "state": {
    "materials": [
      {
        "id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T12:48:36.946Z"
      }
    ],
    "material_versions": [
      {
        "id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T12:48:36.946Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "current_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
      "revision": "1"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
        "operation": "material.create",
        "payload": "{\"request_id\":\"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T12:48:36.946Z\",\"updated_at\":\"2026-10-24T12:48:36.946Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"1\"}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "1"
      }
    ],
    "business_records": [],
    "business_revisions": [],
    "business_heads": [],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(2,'{
  "format_version": 2,
  "schema_version": 4,
  "sequence": "2",
  "request_id": "6deb32c0-f79f-4931-90bf-2dc7e7a0cfac",
  "recorded_at": "2026-10-24T12:48:36.976Z",
  "state": {
    "materials": [
      {
        "id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T12:48:36.946Z"
      }
    ],
    "material_versions": [
      {
        "id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T12:48:36.946Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "current_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061"
      }
    ],
    "material_prices": [],
    "catalog_state": {
      "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
      "revision": "2"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
        "operation": "material.create",
        "payload": "{\"request_id\":\"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T12:48:36.946Z\",\"updated_at\":\"2026-10-24T12:48:36.946Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"1\"}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "1"
      },
      {
        "request_id": "6deb32c0-f79f-4931-90bf-2dc7e7a0cfac",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"6deb32c0-f79f-4931-90bf-2dc7e7a0cfac\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:36.976Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:36.976Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"2\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "2"
      }
    ],
    "business_records": [
      {
        "id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:36.976Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:36.976Z",
        "seq": "2"
      }
    ],
    "business_heads": [
      {
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "current_revision_id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c"
      }
    ],
    "business_confirmations": [],
    "business_catalog_price_results": [],
    "business_corrections": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(3,'{
  "format_version": 2,
  "schema_version": 4,
  "sequence": "3",
  "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
  "recorded_at": "2026-10-24T12:48:37.007Z",
  "state": {
    "materials": [
      {
        "id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T12:48:36.946Z"
      }
    ],
    "material_versions": [
      {
        "id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T12:48:36.946Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "current_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061"
      }
    ],
    "material_prices": [
      {
        "id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      }
    ],
    "catalog_state": {
      "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
      "revision": "3"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
        "operation": "material.create",
        "payload": "{\"request_id\":\"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T12:48:36.946Z\",\"updated_at\":\"2026-10-24T12:48:36.946Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"1\"}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "1"
      },
      {
        "request_id": "6deb32c0-f79f-4931-90bf-2dc7e7a0cfac",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"6deb32c0-f79f-4931-90bf-2dc7e7a0cfac\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:36.976Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:36.976Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"2\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "2"
      },
      {
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "operation": "business.confirm",
        "payload": "[\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",{\"meta\":{\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"2\"},\"expected_head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:37.007Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"effective_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"revision_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"d8c2836a-6b6b-4b49-81eb-dbfc06f4d928\",\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\"}},\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"3\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "3"
      }
    ],
    "business_records": [
      {
        "id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:36.976Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      }
    ],
    "business_revisions": [
      {
        "id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:36.976Z",
        "seq": "2"
      },
      {
        "id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      }
    ],
    "business_heads": [
      {
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "current_revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340"
      }
    ],
    "business_confirmations": [
      {
        "id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "price_id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928"
      }
    ],
    "business_corrections": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(4,'{
  "format_version": 2,
  "schema_version": 4,
  "sequence": "4",
  "request_id": "ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8",
  "recorded_at": "2026-10-24T12:48:37.052Z",
  "state": {
    "materials": [
      {
        "id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T12:48:36.946Z"
      }
    ],
    "material_versions": [
      {
        "id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T12:48:36.946Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "current_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061"
      }
    ],
    "material_prices": [
      {
        "id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      }
    ],
    "catalog_state": {
      "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
      "revision": "4"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
        "operation": "material.create",
        "payload": "{\"request_id\":\"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T12:48:36.946Z\",\"updated_at\":\"2026-10-24T12:48:36.946Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"1\"}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "1"
      },
      {
        "request_id": "6deb32c0-f79f-4931-90bf-2dc7e7a0cfac",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"6deb32c0-f79f-4931-90bf-2dc7e7a0cfac\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:36.976Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:36.976Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"2\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "2"
      },
      {
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "operation": "business.confirm",
        "payload": "[\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",{\"meta\":{\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"2\"},\"expected_head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:37.007Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"effective_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"revision_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"d8c2836a-6b6b-4b49-81eb-dbfc06f4d928\",\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\"}},\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"3\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "3"
      },
      {
        "request_id": "ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"kind\":\"catalog_price\",\"head_id\":\"015584a6-6137-4ca7-9834-5669423f573e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:37.052Z\",\"updated_at\":\"2026-10-24T12:48:37.052Z\",\"corrects_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"confirmation_id\":null,\"effective_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"015584a6-6137-4ca7-9834-5669423f573e\",\"record_id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T12:48:37.052Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"4\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "4"
      }
    ],
    "business_records": [
      {
        "id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:36.976Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:37.052Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3"
      }
    ],
    "business_revisions": [
      {
        "id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:36.976Z",
        "seq": "2"
      },
      {
        "id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      },
      {
        "id": "015584a6-6137-4ca7-9834-5669423f573e",
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T12:48:37.052Z",
        "seq": "4"
      }
    ],
    "business_heads": [
      {
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "current_revision_id": "015584a6-6137-4ca7-9834-5669423f573e"
      },
      {
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "current_revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340"
      }
    ],
    "business_confirmations": [
      {
        "id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "price_id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928"
      }
    ],
    "business_corrections": []
  }
}
',1);
INSERT INTO "snapshot_outbox" VALUES(5,'{
  "format_version": 2,
  "schema_version": 4,
  "sequence": "5",
  "request_id": "defd0520-5c9f-443a-8379-c86b660e578d",
  "recorded_at": "2026-10-24T12:48:37.085Z",
  "state": {
    "materials": [
      {
        "id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "created_at": "2026-10-24T12:48:36.946Z"
      }
    ],
    "material_versions": [
      {
        "id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "name": "가상 이전 가격 재료",
        "unit": "g",
        "canonical_unit": "g",
        "dimension": "mass",
        "kind": "raw",
        "description": "형식 보존 검증용",
        "archived": false,
        "recorded_at": "2026-10-24T12:48:36.946Z",
        "seq": "1",
        "source": "catalog"
      }
    ],
    "material_heads": [
      {
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "current_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061"
      }
    ],
    "material_prices": [
      {
        "id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "quantity": "1000",
        "amount": "10",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      },
      {
        "id": "86953cc1-321f-477f-bdd9-df68917ce95f",
        "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
        "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
        "quantity": "1000",
        "amount": "30",
        "unit": "g",
        "input_quantity": "1",
        "input_unit": "kg",
        "recorded_at": "2026-10-24T12:48:37.085Z",
        "seq": "5"
      }
    ],
    "catalog_state": {
      "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
      "revision": "5"
    },
    "creation_requests": [],
    "mutation_requests": [
      {
        "request_id": "ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6",
        "operation": "material.create",
        "payload": "{\"request_id\":\"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"0\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\"}",
        "response": "{\"id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"name\":\"가상 이전 가격 재료\",\"unit\":\"g\",\"canonical_unit\":\"g\",\"dimension\":\"mass\",\"kind\":\"raw\",\"description\":\"형식 보존 검증용\",\"archived\":false,\"created_at\":\"2026-10-24T12:48:36.946Z\",\"updated_at\":\"2026-10-24T12:48:36.946Z\",\"price\":{\"source\":\"missing\",\"source_id\":null,\"amount\":null,\"quantity\":null,\"reason\":\"등록한 가격이 없습니다\"},\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"1\"}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "1"
      },
      {
        "request_id": "6deb32c0-f79f-4931-90bf-2dc7e7a0cfac",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"6deb32c0-f79f-4931-90bf-2dc7e7a0cfac\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"1\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"corrects_confirmation_id\":null}",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:36.976Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":null,\"effective_confirmation_id\":null,\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:36.976Z\",\"seq\":\"2\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"2\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "2"
      },
      {
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "operation": "business.confirm",
        "payload": "[\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",{\"meta\":{\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"2\"},\"expected_head_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"kind\":\"catalog_price\",\"head_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T12:48:36.976Z\",\"updated_at\":\"2026-10-24T12:48:37.007Z\",\"corrects_confirmation_id\":null,\"confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"effective_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"previous_revision_id\":\"fd9eb3af-68bb-48c5-89b1-33a998446b4c\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"10\",\"note\":\"\"}},\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\"},\"confirmation\":{\"id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"record_id\":\"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5\",\"revision_id\":\"30055d4e-3231-41c0-8ecd-8909bb23d340\",\"request_id\":\"e4ed39ce-150c-4958-bb70-64bd1d6b42d2\",\"recorded_at\":\"2026-10-24T12:48:37.007Z\",\"seq\":\"3\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"d8c2836a-6b6b-4b49-81eb-dbfc06f4d928\",\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\"}},\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"3\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "3"
      },
      {
        "request_id": "ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8",
        "operation": "business.create",
        "payload": "{\"meta\":{\"request_id\":\"ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"3\"},\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"corrects_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\"}",
        "response": "{\"result\":{\"item\":{\"id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"kind\":\"catalog_price\",\"head_id\":\"015584a6-6137-4ca7-9834-5669423f573e\",\"status\":\"draft\",\"created_at\":\"2026-10-24T12:48:37.052Z\",\"updated_at\":\"2026-10-24T12:48:37.052Z\",\"corrects_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"confirmation_id\":null,\"effective_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"015584a6-6137-4ca7-9834-5669423f573e\",\"record_id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"previous_revision_id\":null,\"status\":\"draft\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T12:48:37.052Z\",\"seq\":\"4\"},\"confirmation\":null,\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"4\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "4"
      },
      {
        "request_id": "defd0520-5c9f-443a-8379-c86b660e578d",
        "operation": "business.confirm",
        "payload": "[\"48cfec56-b857-4d04-a4af-17b16345a0a2\",{\"meta\":{\"request_id\":\"defd0520-5c9f-443a-8379-c86b660e578d\",\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"expected_revision\":\"4\"},\"expected_head_id\":\"015584a6-6137-4ca7-9834-5669423f573e\"}]",
        "response": "{\"result\":{\"item\":{\"id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"kind\":\"catalog_price\",\"head_id\":\"1c715cb8-2e2f-4cc5-9a75-12753642912e\",\"status\":\"confirmed\",\"created_at\":\"2026-10-24T12:48:37.052Z\",\"updated_at\":\"2026-10-24T12:48:37.085Z\",\"corrects_confirmation_id\":\"03b64304-3cca-4af5-9aac-2ed9b8122ae3\",\"confirmation_id\":\"85c27d33-b438-47f0-9520-59fefd1d84f2\",\"effective_confirmation_id\":\"85c27d33-b438-47f0-9520-59fefd1d84f2\",\"issue_count\":0,\"material_name\":\"가상 이전 가격 재료\"},\"head\":{\"id\":\"1c715cb8-2e2f-4cc5-9a75-12753642912e\",\"record_id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"previous_revision_id\":\"015584a6-6137-4ca7-9834-5669423f573e\",\"status\":\"confirmed\",\"payload\":{\"kind\":\"catalog_price\",\"version\":1,\"data\":{\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\",\"quantity\":\"1\",\"unit\":\"kg\",\"amount\":\"30\",\"note\":\"가상 이전 금액 정정\"}},\"recorded_at\":\"2026-10-24T12:48:37.085Z\",\"seq\":\"5\"},\"confirmation\":{\"id\":\"85c27d33-b438-47f0-9520-59fefd1d84f2\",\"record_id\":\"48cfec56-b857-4d04-a4af-17b16345a0a2\",\"revision_id\":\"1c715cb8-2e2f-4cc5-9a75-12753642912e\",\"request_id\":\"defd0520-5c9f-443a-8379-c86b660e578d\",\"recorded_at\":\"2026-10-24T12:48:37.085Z\",\"seq\":\"5\",\"result\":{\"kind\":\"catalog_price\",\"price_id\":\"86953cc1-321f-477f-bdd9-df68917ce95f\",\"material_id\":\"2589dd8e-5026-4f97-897c-ae046c15a4b2\",\"material_version_id\":\"0a392cc4-ff12-4e38-a7c8-ff0168285061\"}},\"issues\":[]},\"state\":{\"epoch\":\"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7\",\"revision\":\"5\"}}",
        "epoch": "c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7",
        "sequence": "5"
      }
    ],
    "business_records": [
      {
        "id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:36.976Z",
        "created_sequence": "2",
        "corrects_confirmation_id": null
      },
      {
        "id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "kind": "catalog_price",
        "created_at": "2026-10-24T12:48:37.052Z",
        "created_sequence": "4",
        "corrects_confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3"
      }
    ],
    "business_revisions": [
      {
        "id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:36.976Z",
        "seq": "2"
      },
      {
        "id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "previous_revision_id": "fd9eb3af-68bb-48c5-89b1-33a998446b4c",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "10",
            "note": ""
          }
        },
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      },
      {
        "id": "015584a6-6137-4ca7-9834-5669423f573e",
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "previous_revision_id": null,
        "status": "draft",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T12:48:37.052Z",
        "seq": "4"
      },
      {
        "id": "1c715cb8-2e2f-4cc5-9a75-12753642912e",
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "previous_revision_id": "015584a6-6137-4ca7-9834-5669423f573e",
        "status": "confirmed",
        "payload": {
          "kind": "catalog_price",
          "version": 1,
          "data": {
            "material_id": "2589dd8e-5026-4f97-897c-ae046c15a4b2",
            "material_version_id": "0a392cc4-ff12-4e38-a7c8-ff0168285061",
            "quantity": "1",
            "unit": "kg",
            "amount": "30",
            "note": "가상 이전 금액 정정"
          }
        },
        "recorded_at": "2026-10-24T12:48:37.085Z",
        "seq": "5"
      }
    ],
    "business_heads": [
      {
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "current_revision_id": "1c715cb8-2e2f-4cc5-9a75-12753642912e"
      },
      {
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "current_revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340"
      }
    ],
    "business_confirmations": [
      {
        "id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "record_id": "cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",
        "revision_id": "30055d4e-3231-41c0-8ecd-8909bb23d340",
        "request_id": "e4ed39ce-150c-4958-bb70-64bd1d6b42d2",
        "recorded_at": "2026-10-24T12:48:37.007Z",
        "seq": "3"
      },
      {
        "id": "85c27d33-b438-47f0-9520-59fefd1d84f2",
        "record_id": "48cfec56-b857-4d04-a4af-17b16345a0a2",
        "revision_id": "1c715cb8-2e2f-4cc5-9a75-12753642912e",
        "request_id": "defd0520-5c9f-443a-8379-c86b660e578d",
        "recorded_at": "2026-10-24T12:48:37.085Z",
        "seq": "5"
      }
    ],
    "business_catalog_price_results": [
      {
        "confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "price_id": "d8c2836a-6b6b-4b49-81eb-dbfc06f4d928"
      },
      {
        "confirmation_id": "85c27d33-b438-47f0-9520-59fefd1d84f2",
        "price_id": "86953cc1-321f-477f-bdd9-df68917ce95f"
      }
    ],
    "business_corrections": [
      {
        "target_confirmation_id": "03b64304-3cca-4af5-9aac-2ed9b8122ae3",
        "replacement_confirmation_id": "85c27d33-b438-47f0-9520-59fefd1d84f2"
      }
    ]
  }
}
',1);
INSERT INTO "material_versions" VALUES('0a392cc4-ff12-4e38-a7c8-ff0168285061','2589dd8e-5026-4f97-897c-ae046c15a4b2','가상 이전 가격 재료','g','g','mass','raw','형식 보존 검증용',0,'2026-10-24T12:48:36.946Z',1,'catalog');
INSERT INTO "material_heads" VALUES('2589dd8e-5026-4f97-897c-ae046c15a4b2','0a392cc4-ff12-4e38-a7c8-ff0168285061');
INSERT INTO "material_prices" VALUES('d8c2836a-6b6b-4b49-81eb-dbfc06f4d928','2589dd8e-5026-4f97-897c-ae046c15a4b2','0a392cc4-ff12-4e38-a7c8-ff0168285061',1000000000,10000000,'g','1','kg','2026-10-24T12:48:37.007Z',3);
INSERT INTO "material_prices" VALUES('86953cc1-321f-477f-bdd9-df68917ce95f','2589dd8e-5026-4f97-897c-ae046c15a4b2','0a392cc4-ff12-4e38-a7c8-ff0168285061',1000000000,30000000,'g','1','kg','2026-10-24T12:48:37.085Z',5);
INSERT INTO "mutation_requests" VALUES('ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6','material.create','{"request_id":"ec3f1dc9-2aa6-4d8e-beaa-4b25249b61b6","epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","expected_revision":"0","name":"가상 이전 가격 재료","unit":"g","kind":"raw","description":"형식 보존 검증용"}','{"id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","name":"가상 이전 가격 재료","unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"형식 보존 검증용","archived":false,"created_at":"2026-10-24T12:48:36.946Z","updated_at":"2026-10-24T12:48:36.946Z","price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록한 가격이 없습니다"},"epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","revision":"1"}','c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',1);
INSERT INTO "mutation_requests" VALUES('6deb32c0-f79f-4931-90bf-2dc7e7a0cfac','business.create','{"meta":{"request_id":"6deb32c0-f79f-4931-90bf-2dc7e7a0cfac","epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","expected_revision":"1"},"payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"10","note":""}},"corrects_confirmation_id":null}','{"result":{"item":{"id":"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5","kind":"catalog_price","head_id":"fd9eb3af-68bb-48c5-89b1-33a998446b4c","status":"draft","created_at":"2026-10-24T12:48:36.976Z","updated_at":"2026-10-24T12:48:36.976Z","corrects_confirmation_id":null,"confirmation_id":null,"effective_confirmation_id":null,"issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"fd9eb3af-68bb-48c5-89b1-33a998446b4c","record_id":"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5","previous_revision_id":null,"status":"draft","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"10","note":""}},"recorded_at":"2026-10-24T12:48:36.976Z","seq":"2"},"confirmation":null,"issues":[]},"state":{"epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","revision":"2"}}','c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',2);
INSERT INTO "mutation_requests" VALUES('e4ed39ce-150c-4958-bb70-64bd1d6b42d2','business.confirm','["cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5",{"meta":{"request_id":"e4ed39ce-150c-4958-bb70-64bd1d6b42d2","epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","expected_revision":"2"},"expected_head_id":"fd9eb3af-68bb-48c5-89b1-33a998446b4c"}]','{"result":{"item":{"id":"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5","kind":"catalog_price","head_id":"30055d4e-3231-41c0-8ecd-8909bb23d340","status":"confirmed","created_at":"2026-10-24T12:48:36.976Z","updated_at":"2026-10-24T12:48:37.007Z","corrects_confirmation_id":null,"confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","effective_confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"30055d4e-3231-41c0-8ecd-8909bb23d340","record_id":"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5","previous_revision_id":"fd9eb3af-68bb-48c5-89b1-33a998446b4c","status":"confirmed","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"10","note":""}},"recorded_at":"2026-10-24T12:48:37.007Z","seq":"3"},"confirmation":{"id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","record_id":"cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5","revision_id":"30055d4e-3231-41c0-8ecd-8909bb23d340","request_id":"e4ed39ce-150c-4958-bb70-64bd1d6b42d2","recorded_at":"2026-10-24T12:48:37.007Z","seq":"3","result":{"kind":"catalog_price","price_id":"d8c2836a-6b6b-4b49-81eb-dbfc06f4d928","material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061"}},"issues":[]},"state":{"epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","revision":"3"}}','c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',3);
INSERT INTO "mutation_requests" VALUES('ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8','business.create','{"meta":{"request_id":"ebd82fcb-dc07-4c7b-a6b7-2bbdcac160e8","epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","expected_revision":"3"},"payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"corrects_confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3"}','{"result":{"item":{"id":"48cfec56-b857-4d04-a4af-17b16345a0a2","kind":"catalog_price","head_id":"015584a6-6137-4ca7-9834-5669423f573e","status":"draft","created_at":"2026-10-24T12:48:37.052Z","updated_at":"2026-10-24T12:48:37.052Z","corrects_confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","confirmation_id":null,"effective_confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"015584a6-6137-4ca7-9834-5669423f573e","record_id":"48cfec56-b857-4d04-a4af-17b16345a0a2","previous_revision_id":null,"status":"draft","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"recorded_at":"2026-10-24T12:48:37.052Z","seq":"4"},"confirmation":null,"issues":[]},"state":{"epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","revision":"4"}}','c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',4);
INSERT INTO "mutation_requests" VALUES('defd0520-5c9f-443a-8379-c86b660e578d','business.confirm','["48cfec56-b857-4d04-a4af-17b16345a0a2",{"meta":{"request_id":"defd0520-5c9f-443a-8379-c86b660e578d","epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","expected_revision":"4"},"expected_head_id":"015584a6-6137-4ca7-9834-5669423f573e"}]','{"result":{"item":{"id":"48cfec56-b857-4d04-a4af-17b16345a0a2","kind":"catalog_price","head_id":"1c715cb8-2e2f-4cc5-9a75-12753642912e","status":"confirmed","created_at":"2026-10-24T12:48:37.052Z","updated_at":"2026-10-24T12:48:37.085Z","corrects_confirmation_id":"03b64304-3cca-4af5-9aac-2ed9b8122ae3","confirmation_id":"85c27d33-b438-47f0-9520-59fefd1d84f2","effective_confirmation_id":"85c27d33-b438-47f0-9520-59fefd1d84f2","issue_count":0,"material_name":"가상 이전 가격 재료"},"head":{"id":"1c715cb8-2e2f-4cc5-9a75-12753642912e","record_id":"48cfec56-b857-4d04-a4af-17b16345a0a2","previous_revision_id":"015584a6-6137-4ca7-9834-5669423f573e","status":"confirmed","payload":{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}},"recorded_at":"2026-10-24T12:48:37.085Z","seq":"5"},"confirmation":{"id":"85c27d33-b438-47f0-9520-59fefd1d84f2","record_id":"48cfec56-b857-4d04-a4af-17b16345a0a2","revision_id":"1c715cb8-2e2f-4cc5-9a75-12753642912e","request_id":"defd0520-5c9f-443a-8379-c86b660e578d","recorded_at":"2026-10-24T12:48:37.085Z","seq":"5","result":{"kind":"catalog_price","price_id":"86953cc1-321f-477f-bdd9-df68917ce95f","material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061"}},"issues":[]},"state":{"epoch":"c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7","revision":"5"}}','c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',5);
INSERT INTO "catalog_state" VALUES(1,'c14d6897-0c1a-4f38-9b3b-5d8e6dd970c7',5);
INSERT INTO "business_records" VALUES('cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5','catalog_price','2026-10-24T12:48:36.976Z',2,NULL);
INSERT INTO "business_records" VALUES('48cfec56-b857-4d04-a4af-17b16345a0a2','catalog_price','2026-10-24T12:48:37.052Z',4,'03b64304-3cca-4af5-9aac-2ed9b8122ae3');
INSERT INTO "business_revisions" VALUES('fd9eb3af-68bb-48c5-89b1-33a998446b4c','cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5',NULL,'draft',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"10","note":""}}','2026-10-24T12:48:36.976Z',2);
INSERT INTO "business_revisions" VALUES('30055d4e-3231-41c0-8ecd-8909bb23d340','cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5','fd9eb3af-68bb-48c5-89b1-33a998446b4c','confirmed',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"10","note":""}}','2026-10-24T12:48:37.007Z',3);
INSERT INTO "business_revisions" VALUES('015584a6-6137-4ca7-9834-5669423f573e','48cfec56-b857-4d04-a4af-17b16345a0a2',NULL,'draft',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}}','2026-10-24T12:48:37.052Z',4);
INSERT INTO "business_revisions" VALUES('1c715cb8-2e2f-4cc5-9a75-12753642912e','48cfec56-b857-4d04-a4af-17b16345a0a2','015584a6-6137-4ca7-9834-5669423f573e','confirmed',1,'{"kind":"catalog_price","version":1,"data":{"material_id":"2589dd8e-5026-4f97-897c-ae046c15a4b2","material_version_id":"0a392cc4-ff12-4e38-a7c8-ff0168285061","quantity":"1","unit":"kg","amount":"30","note":"가상 이전 금액 정정"}}','2026-10-24T12:48:37.085Z',5);
INSERT INTO "business_heads" VALUES('cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5','30055d4e-3231-41c0-8ecd-8909bb23d340');
INSERT INTO "business_heads" VALUES('48cfec56-b857-4d04-a4af-17b16345a0a2','1c715cb8-2e2f-4cc5-9a75-12753642912e');
INSERT INTO "business_confirmations" VALUES('03b64304-3cca-4af5-9aac-2ed9b8122ae3','cb622ee9-7d1a-44b7-a0eb-fe902c99f4f5','30055d4e-3231-41c0-8ecd-8909bb23d340','e4ed39ce-150c-4958-bb70-64bd1d6b42d2','2026-10-24T12:48:37.007Z',3);
INSERT INTO "business_confirmations" VALUES('85c27d33-b438-47f0-9520-59fefd1d84f2','48cfec56-b857-4d04-a4af-17b16345a0a2','1c715cb8-2e2f-4cc5-9a75-12753642912e','defd0520-5c9f-443a-8379-c86b660e578d','2026-10-24T12:48:37.085Z',5);
INSERT INTO "business_catalog_price_results" VALUES('03b64304-3cca-4af5-9aac-2ed9b8122ae3','d8c2836a-6b6b-4b49-81eb-dbfc06f4d928');
INSERT INTO "business_catalog_price_results" VALUES('85c27d33-b438-47f0-9520-59fefd1d84f2','86953cc1-321f-477f-bdd9-df68917ce95f');
INSERT INTO "business_corrections" VALUES('03b64304-3cca-4af5-9aac-2ed9b8122ae3','85c27d33-b438-47f0-9520-59fefd1d84f2');
PRAGMA user_version=4;
