use super::{Store, StoreError, catalog, holding, snapshot, unavailable};
use crate::{
    business::BusinessMeta,
    catalog::{ArchiveFilter, MaterialVersion, MutationMeta},
    domain::{Money, Quantity, Unit, UnitPrice},
    recipe::*,
};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

fn validation(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(error.to_string())
}
fn stale() -> StoreError {
    StoreError::StaleState("품목 또는 레시피가 변경되었습니다. 최신 내용을 확인하세요".into())
}
fn normalize_meta(input: &mut BusinessMeta) -> Result<MutationMeta, StoreError> {
    let mut meta: MutationMeta = input.clone().into();
    catalog::normalize_meta(&mut meta)?;
    input.request_id = meta.request_id.clone();
    input.epoch = meta.epoch.clone();
    input.expected_revision = meta.expected_revision.clone();
    Ok(meta)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VersionRow {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub material_unit: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub basis_count: String,
    pub reference_output: Option<RecipeReferenceOutput>,
    pub selling_price: Option<String>,
    pub cooking_description: String,
    pub process: String,
    pub reference_time: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LineRow {
    pub recipe_version_id: String,
    pub line: RecipeLine,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head {
    pub material_id: String,
    pub current_version_id: String,
}
pub(super) struct SnapshotRows {
    pub versions: Vec<VersionRow>,
    pub lines: Vec<LineRow>,
    pub heads: Vec<Head>,
}

fn number_error(
    index: usize,
    error: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Integer,
        Box::new(error),
    )
}
fn quantity_at(row: &Row<'_>, index: usize) -> rusqlite::Result<String> {
    Quantity::from_micros(row.get(index)?)
        .map(|q| q.decimal())
        .map_err(|e| number_error(index, e))
}
fn version_row(row: &Row<'_>) -> rusqlite::Result<VersionRow> {
    let output = row
        .get::<_, Option<i64>>(8)?
        .map(|q| {
            Ok::<_, rusqlite::Error>(RecipeReferenceOutput {
                quantity: Quantity::from_micros(q)
                    .map_err(|e| number_error(8, e))?
                    .decimal(),
                unit: row.get(9)?,
                input_quantity: row.get(10)?,
                input_unit: row.get(11)?,
            })
        })
        .transpose()?;
    let price = row
        .get::<_, Option<i64>>(12)?
        .map(|a| {
            Money::from_micros(a)
                .map(|m| m.decimal())
                .map_err(|e| number_error(12, e))
        })
        .transpose()?;
    Ok(VersionRow {
        id: row.get(0)?,
        material_id: row.get(1)?,
        material_version_id: row.get(2)?,
        material_name: row.get(3)?,
        material_unit: row.get(4)?,
        previous_version_id: row.get(5)?,
        copied_from_version_id: row.get(6)?,
        basis_count: row.get::<_, i64>(7)?.to_string(),
        reference_output: output,
        selling_price: price,
        cooking_description: row.get(13)?,
        process: row.get(14)?,
        reference_time: row.get(15)?,
        recorded_at: row.get(16)?,
        seq: row.get::<_, i64>(17)?.to_string(),
    })
}
const VERSION_COLUMNS: &str = "r.id,r.material_id,r.material_version_id,m.name,m.unit,r.previous_version_id,r.copied_from_version_id,r.basis_count,r.reference_quantity_micros,r.reference_unit,r.reference_input_quantity,r.reference_input_unit,r.selling_price_micros,r.cooking_description,r.process,r.reference_time,r.recorded_at,r.sequence";

fn versions(
    connection: &Connection,
    material: Option<&str>,
    id: Option<&str>,
) -> Result<Vec<VersionRow>, StoreError> {
    connection.prepare(&format!("SELECT {VERSION_COLUMNS} FROM recipe_versions r JOIN material_versions m ON m.id=r.material_version_id AND m.material_id=r.material_id WHERE (?1 IS NULL OR r.material_id=?1) AND (?2 IS NULL OR r.id=?2) ORDER BY r.sequence,r.id")).map_err(unavailable)?
        .query_map(params![material,id], version_row).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}
fn current(connection: &Connection, material: &str) -> Result<Option<VersionRow>, StoreError> {
    connection.query_row(&format!("SELECT {VERSION_COLUMNS} FROM recipe_heads h JOIN recipe_versions r ON r.id=h.current_version_id AND r.material_id=h.material_id JOIN material_versions m ON m.id=r.material_version_id AND m.material_id=r.material_id WHERE h.material_id=?1"),[material],version_row).optional().map_err(unavailable)
}
fn lines(connection: &Connection, version: Option<&str>) -> Result<Vec<LineRow>, StoreError> {
    connection.prepare("SELECT l.recipe_version_id,l.id,l.position,l.material_id,l.material_version_id,m.name,l.quantity_micros,l.unit,l.input_quantity,l.input_unit,l.note FROM recipe_lines l JOIN material_versions m ON m.id=l.material_version_id AND m.material_id=l.material_id WHERE (?1 IS NULL OR l.recipe_version_id=?1) ORDER BY l.recipe_version_id,l.position").map_err(unavailable)?
        .query_map([version], |r| Ok(LineRow {
            recipe_version_id: r.get(0)?,
            line: RecipeLine {
                id:r.get(1)?,position:r.get::<_,i64>(2)?.to_string(),material_id:r.get(3)?,material_version_id:r.get(4)?,material_name:r.get(5)?,quantity:quantity_at(r,6)?,unit:r.get(7)?,input_quantity:r.get(8)?,input_unit:r.get(9)?,note:r.get(10)?,
            },
        })).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)
}
fn with_lines(row: VersionRow, lines: Vec<RecipeLine>) -> RecipeVersion {
    RecipeVersion {
        id: row.id,
        material_id: row.material_id,
        material_version_id: row.material_version_id,
        material_name: row.material_name,
        material_unit: row.material_unit,
        previous_version_id: row.previous_version_id,
        copied_from_version_id: row.copied_from_version_id,
        basis_count: row.basis_count,
        reference_output: row.reference_output,
        selling_price: row.selling_price,
        cooking_description: row.cooking_description,
        process: row.process,
        reference_time: row.reference_time,
        lines,
        recorded_at: row.recorded_at,
        seq: row.seq,
    }
}
fn full_version(connection: &Connection, row: VersionRow) -> Result<RecipeVersion, StoreError> {
    let rows = lines(connection, Some(&row.id))?;
    Ok(with_lines(row, rows.into_iter().map(|r| r.line).collect()))
}
fn material_version(
    connection: &Connection,
    material: &str,
    version: &str,
) -> Result<MaterialVersion, StoreError> {
    catalog::versions(connection, Some(material))?
        .into_iter()
        .find(|v| v.id == version)
        .ok_or_else(|| validation("참조한 품목 버전이 해당 품목에 없습니다"))
}
fn required_owner(
    connection: &Connection,
    id: &str,
    expected: &str,
) -> Result<MaterialVersion, StoreError> {
    let owner = catalog::current_version(connection, id)?
        .ok_or_else(|| validation("레시피 소유 품목을 찾을 수 없습니다"))?;
    if owner.id != expected {
        return Err(stale());
    }
    Ok(owner)
}

fn ensure_acyclic(graph: &HashMap<String, Vec<String>>) -> Result<(), StoreError> {
    let mut finished = HashSet::new();
    for start in graph.keys() {
        if finished.contains(start) {
            continue;
        }
        let mut active = HashSet::new();
        let mut stack = vec![(start.as_str(), false)];
        while let Some((node, exit)) = stack.pop() {
            if exit {
                active.remove(node);
                finished.insert(node.to_owned());
                continue;
            }
            if finished.contains(node) {
                continue;
            }
            if !active.insert(node) {
                return Err(validation(
                    "레시피가 자기 자신 또는 순환하는 품목을 참조합니다",
                ));
            }
            stack.push((node, true));
            if let Some(children) = graph.get(node) {
                for child in children.iter().rev() {
                    if active.contains(child.as_str()) {
                        return Err(validation(
                            "레시피가 자기 자신 또는 순환하는 품목을 참조합니다",
                        ));
                    }
                    if !finished.contains(child) {
                        stack.push((child, false));
                    }
                }
            }
        }
    }
    Ok(())
}
fn check_candidate(
    connection: &Connection,
    owner: &str,
    components: &[RecipeLine],
) -> Result<(), StoreError> {
    let edges=connection.prepare("SELECT h.material_id,l.material_id FROM recipe_heads h JOIN recipe_lines l ON l.recipe_version_id=h.current_version_id ORDER BY h.material_id,l.position").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let mut graph = HashMap::<String, Vec<String>>::new();
    for (parent, child) in edges {
        graph.entry(parent).or_default().push(child);
    }
    graph.insert(
        owner.into(),
        components.iter().map(|l| l.material_id.clone()).collect(),
    );
    ensure_acyclic(&graph)
}

fn prepare_version(
    connection: &Connection,
    owner: &MaterialVersion,
    data: RecipeDataInput,
    previous: Option<String>,
    copied: Option<String>,
    seq: i64,
    time: &str,
) -> Result<RecipeVersion, StoreError> {
    let reference_output = data
        .reference_output
        .map(|r| {
            let unit = Unit::parse(&r.unit).map_err(validation)?;
            if !unit.same_dimension(&Unit::parse(&owner.unit).map_err(unavailable)?) {
                return Err(validation(
                    "참고 생산량은 소유 품목과 같은 차원 단위를 사용하세요",
                ));
            }
            Ok(RecipeReferenceOutput {
                quantity: unit
                    .to_canonical(Quantity::parse(&r.quantity).map_err(validation)?)
                    .map_err(validation)?
                    .decimal(),
                unit: unit.canonical().into(),
                input_quantity: r.quantity,
                input_unit: r.unit,
            })
        })
        .transpose()?;
    let mut prepared = vec![];
    for (position, line) in data.lines.into_iter().enumerate() {
        let material = material_version(connection, &line.material_id, &line.material_version_id)?;
        let unit = Unit::parse(&line.unit).map_err(validation)?;
        if !unit.same_dimension(&Unit::parse(&material.unit).map_err(unavailable)?) {
            return Err(validation(
                "구성 수량은 참조 품목과 같은 차원 단위를 사용하세요",
            ));
        }
        prepared.push(RecipeLine {
            id: Uuid::new_v4().to_string(),
            position: position.to_string(),
            material_id: line.material_id,
            material_version_id: line.material_version_id,
            material_name: material.name,
            quantity: unit
                .to_canonical(Quantity::parse(&line.quantity).map_err(validation)?)
                .map_err(validation)?
                .decimal(),
            unit: unit.canonical().into(),
            input_quantity: line.quantity,
            input_unit: line.unit,
            note: line.note,
        });
    }
    check_candidate(connection, &owner.material_id, &prepared)?;
    Ok(RecipeVersion {
        id: Uuid::new_v4().to_string(),
        material_id: owner.material_id.clone(),
        material_version_id: owner.id.clone(),
        material_name: owner.name.clone(),
        material_unit: owner.unit.clone(),
        previous_version_id: previous,
        copied_from_version_id: copied,
        basis_count: data.basis_count,
        reference_output,
        selling_price: data
            .selling_price
            .map(|p| Money::parse(&p).map(|m| m.decimal()).map_err(validation))
            .transpose()?,
        cooking_description: data.cooking_description,
        process: data.process,
        reference_time: data.reference_time,
        lines: prepared,
        recorded_at: time.into(),
        seq: seq.to_string(),
    })
}
fn insert_version(connection: &Connection, version: &RecipeVersion) -> Result<(), StoreError> {
    let output = version.reference_output.as_ref();
    let quantity = output
        .map(|o| {
            Quantity::parse(&o.quantity)
                .map(|q| q.micros())
                .map_err(unavailable)
        })
        .transpose()?;
    let price = version
        .selling_price
        .as_ref()
        .map(|p| Money::parse(p).map(|m| m.micros()).map_err(unavailable))
        .transpose()?;
    connection.execute("INSERT INTO recipe_versions(id,material_id,material_version_id,previous_version_id,copied_from_version_id,basis_count,reference_quantity_micros,reference_unit,reference_input_quantity,reference_input_unit,selling_price_micros,cooking_description,process,reference_time,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![version.id,version.material_id,version.material_version_id,version.previous_version_id,version.copied_from_version_id,parse_basis_count(&version.basis_count).map_err(unavailable)?,quantity,output.map(|o|&o.unit),output.map(|o|&o.input_quantity),output.map(|o|&o.input_unit),price,version.cooking_description,version.process,version.reference_time,version.recorded_at,version.seq.parse::<i64>().map_err(unavailable)?]).map_err(unavailable)?;
    for line in &version.lines {
        connection.execute("INSERT INTO recipe_lines(id,recipe_version_id,position,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,note) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![line.id,version.id,line.position.parse::<i64>().map_err(unavailable)?,line.material_id,line.material_version_id,Quantity::parse(&line.quantity).map_err(unavailable)?.micros(),line.unit,line.input_quantity,line.input_unit,line.note]).map_err(unavailable)?;
    }
    connection.execute("INSERT INTO recipe_heads(material_id,current_version_id) VALUES(?1,?2) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![version.material_id,version.id]).map_err(unavailable)?;
    Ok(())
}

impl Store {
    pub fn recipe_create(&mut self, mut input: CreateRecipe) -> Result<RecipeMutation, StoreError> {
        let meta = normalize_meta(&mut input.meta)?;
        normalize_recipe_data(&mut input.data).map_err(validation)?;
        match &mut input.owner {
            RecipeOwnerInput::Existing {
                material_id,
                expected_material_version_id,
            } => {
                *material_id = catalog::uuid(material_id, "소유 품목 ID")?;
                *expected_material_version_id =
                    catalog::uuid(expected_material_version_id, "소유 품목 버전")?;
            }
            RecipeOwnerInput::New {
                name,
                unit,
                kind,
                description,
            } => catalog::normalize_fields(name, unit, kind, description, false)?,
        }
        let payload = serde_json::to_string(&input).map_err(unavailable)?;
        self.mutate(
            Some(&meta),
            &meta.request_id,
            "recipe.create",
            &payload,
            move |tx, seq, time, state| {
                let owner = match input.owner {
                    RecipeOwnerInput::Existing {
                        material_id,
                        expected_material_version_id,
                    } => {
                        let owner =
                            required_owner(tx, &material_id, &expected_material_version_id)?;
                        if current(tx, &material_id)?.is_some() {
                            return Err(stale());
                        }
                        owner
                    }
                    RecipeOwnerInput::New {
                        name,
                        unit,
                        kind,
                        description,
                    } => {
                        let item =
                            catalog::insert_material(tx, name, unit, kind, description, seq, time)?;
                        required_owner(tx, &item.id, &item.version_id)?
                    }
                };
                let version = prepare_version(tx, &owner, input.data, None, None, seq, time)?;
                insert_version(tx, &version)?;
                Ok(RecipeMutation {
                    item: catalog::summary(tx, owner)?,
                    version,
                    state: state.clone(),
                })
            },
        )
    }
    pub fn recipe_revise(
        &mut self,
        id: &str,
        mut input: ReviseRecipe,
    ) -> Result<RecipeMutation, StoreError> {
        let id = catalog::uuid(id, "소유 품목 ID")?;
        let meta = normalize_meta(&mut input.meta)?;
        input.expected_material_version_id =
            catalog::uuid(&input.expected_material_version_id, "소유 품목 버전")?;
        input.expected_recipe_version_id =
            catalog::uuid(&input.expected_recipe_version_id, "레시피 버전")?;
        normalize_recipe_data(&mut input.data).map_err(validation)?;
        let payload = serde_json::to_string(&(&id, &input)).map_err(unavailable)?;
        self.mutate(
            Some(&meta),
            &meta.request_id,
            "recipe.revise",
            &payload,
            move |tx, seq, time, state| {
                let owner = required_owner(tx, &id, &input.expected_material_version_id)?;
                let prior =
                    current(tx, &id)?.ok_or_else(|| validation("레시피를 찾을 수 없습니다"))?;
                if prior.id != input.expected_recipe_version_id {
                    return Err(stale());
                }
                let version =
                    prepare_version(tx, &owner, input.data, Some(prior.id), None, seq, time)?;
                insert_version(tx, &version)?;
                Ok(RecipeMutation {
                    item: catalog::summary(tx, owner)?,
                    version,
                    state: state.clone(),
                })
            },
        )
    }
    pub fn recipe_copy(
        &mut self,
        id: &str,
        mut input: CopyRecipe,
    ) -> Result<RecipeMutation, StoreError> {
        let id = catalog::uuid(id, "원본 품목 ID")?;
        let meta = normalize_meta(&mut input.meta)?;
        input.expected_material_version_id =
            catalog::uuid(&input.expected_material_version_id, "원본 품목 버전")?;
        input.source_recipe_version_id =
            catalog::uuid(&input.source_recipe_version_id, "원본 레시피 버전")?;
        let payload = serde_json::to_string(&(&id, &input)).map_err(unavailable)?;
        self.mutate(Some(&meta),&meta.request_id,"recipe.copy",&payload,move|tx,seq,time,state| {
            let owner=required_owner(tx,&id,&input.expected_material_version_id)?;
            let row=current(tx,&id)?.ok_or_else(||validation("복제할 레시피를 찾을 수 없습니다"))?;
            if row.id!=input.source_recipe_version_id{return Err(stale());}
            let source=full_version(tx,row)?;
            let names=tx.prepare("SELECT v.name FROM material_heads h JOIN material_versions v ON v.id=h.current_version_id AND v.material_id=h.material_id").map_err(unavailable)?
                .query_map([],|r|r.get::<_,String>(0)).map_err(unavailable)?.collect::<rusqlite::Result<HashSet<_>>>().map_err(unavailable)?;
            let name=next_copy_name(&owner.name,&names).map_err(validation)?;
            let item=catalog::insert_material(tx,name,owner.unit,owner.kind,owner.description,seq,time)?;
            let new_owner=required_owner(tx,&item.id,&item.version_id)?;
            let data=RecipeDataInput {basis_count:source.basis_count,reference_output:source.reference_output.map(|o|RecipeReferenceInput{quantity:o.input_quantity,unit:o.input_unit}),selling_price:source.selling_price,cooking_description:source.cooking_description,process:source.process,reference_time:source.reference_time,lines:source.lines.into_iter().map(|l|RecipeLineInput{material_id:l.material_id,material_version_id:l.material_version_id,quantity:l.input_quantity,unit:l.input_unit,note:l.note}).collect()};
            let version=prepare_version(tx,&new_owner,data,None,Some(source.id),seq,time)?;
            insert_version(tx,&version)?;
            Ok(RecipeMutation{item,version,state:state.clone()})
        })
    }
    pub fn recipe_version(
        &self,
        id: &str,
        version: &str,
    ) -> Result<Option<RecipeVersion>, StoreError> {
        let id = catalog::uuid(id, "소유 품목 ID")?;
        let version = catalog::uuid(version, "레시피 버전")?;
        versions(&self.connection, Some(&id), Some(&version))?
            .into_iter()
            .next()
            .map(|r| full_version(&self.connection, r))
            .transpose()
    }
    pub fn recipe_detail(&self, id: &str) -> Result<Option<RecipeDetail>, StoreError> {
        let id = catalog::uuid(id, "소유 품목 ID")?;
        let Some(row) = current(&self.connection, &id)? else {
            return Ok(None);
        };
        let owner = catalog::current_version(&self.connection, &id)?
            .ok_or_else(|| unavailable("레시피 소유 품목이 없습니다"))?;
        let history = versions(&self.connection, Some(&id), None)?
            .into_iter()
            .map(|v| RecipeVersionSummary {
                id: v.id,
                material_version_id: v.material_version_id,
                previous_version_id: v.previous_version_id,
                copied_from_version_id: v.copied_from_version_id,
                basis_count: v.basis_count,
                recorded_at: v.recorded_at,
                seq: v.seq,
            })
            .collect();
        Ok(Some(RecipeDetail {
            item: catalog::summary(&self.connection, owner)?,
            head: full_version(&self.connection, row)?,
            versions: history,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn recipe_list(&self, filter: RecipeFilter) -> Result<RecipeList, StoreError> {
        let query = filter.query.trim();
        if query.chars().count() > 100 || query.chars().any(char::is_control) {
            return Err(validation("검색어는 제어문자 없이 100자 이내로 입력하세요"));
        }
        let archive = match filter.archive {
            ArchiveFilter::Active => Some(false),
            ArchiveFilter::Archived => Some(true),
            ArchiveFilter::All => None,
        };
        let rows=self.connection.prepare("SELECT h.material_id,r.id,r.recorded_at FROM recipe_heads h JOIN recipe_versions r ON r.id=h.current_version_id AND r.material_id=h.material_id JOIN material_heads mh ON mh.material_id=h.material_id JOIN material_versions m ON m.id=mh.current_version_id WHERE instr(m.name,?1)>0 AND (?2 IS NULL OR m.archived=?2) ORDER BY r.sequence,r.id").map_err(unavailable)?
            .query_map(params![query,archive],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
        let items = rows
            .into_iter()
            .map(|(id, recipe_version_id, updated_at)| {
                let owner = catalog::current_version(&self.connection, &id)?
                    .ok_or_else(|| unavailable("레시피 소유 품목이 없습니다"))?;
                Ok(RecipeSummary {
                    item: catalog::summary(&self.connection, owner)?,
                    recipe_version_id,
                    updated_at,
                })
            })
            .collect::<Result<_, StoreError>>()?;
        Ok(RecipeList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn recipe_uses(&self, id: &str) -> Result<Option<RecipeUses>, StoreError> {
        let id = catalog::uuid(id, "구성 품목 ID")?;
        if catalog::current_version(&self.connection, &id)?.is_none() {
            return Ok(None);
        }
        let rows=self.connection.prepare("SELECT DISTINCT h.material_id,r.id,r.recorded_at,r.sequence FROM recipe_heads h JOIN recipe_versions r ON r.id=h.current_version_id JOIN recipe_lines l ON l.recipe_version_id=r.id WHERE l.material_id=?1 ORDER BY r.sequence,r.id").map_err(unavailable)?
            .query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
        let items = rows
            .into_iter()
            .map(|(id, recipe_version_id, updated_at)| {
                let owner = catalog::current_version(&self.connection, &id)?
                    .ok_or_else(|| unavailable("레시피 소유 품목이 없습니다"))?;
                Ok(RecipeSummary {
                    item: catalog::summary(&self.connection, owner)?,
                    recipe_version_id,
                    updated_at,
                })
            })
            .collect::<Result<_, StoreError>>()?;
        Ok(Some(RecipeUses {
            items,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn recipe_cost(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<Option<RecipeCost>, StoreError> {
        let id = catalog::uuid(id, "소유 품목 ID")?;
        let row = match version {
            Some(version) => {
                let version = catalog::uuid(version, "레시피 버전")?;
                versions(&self.connection, Some(&id), Some(&version))?
                    .into_iter()
                    .next()
            }
            None => current(&self.connection, &id)?,
        };
        let Some(row) = row else {
            return Ok(None);
        };
        let recipe = full_version(&self.connection, row)?;
        let mut inputs = vec![];
        let mut rows = vec![];
        let mut missing = vec![];
        for line in &recipe.lines {
            let price = holding::price(
                &self.connection,
                &line.material_id,
                catalog::registered_price(&self.connection, &line.material_id)?,
            )?;
            let unit_price = match (&price.amount, &price.quantity, &price.source_id) {
                (Some(amount), Some(quantity), Some(source)) => Some(
                    UnitPrice::new(
                        Money::parse(amount).map_err(unavailable)?,
                        Quantity::parse(quantity).map_err(unavailable)?,
                        source.clone(),
                    )
                    .map_err(unavailable)?,
                ),
                (None, None, _) => None,
                _ => return Err(unavailable("구성 품목의 가격 출처가 올바르지 않습니다")),
            };
            if unit_price.is_none() {
                missing.push(line.id.clone());
            }
            inputs.push(RecipeCostLineInput {
                numerator: Quantity::parse(&line.quantity).map_err(unavailable)?,
                price: unit_price,
            });
            rows.push(RecipeCostRow {
                line_id: line.id.clone(),
                material_id: line.material_id.clone(),
                material_version_id: line.material_version_id.clone(),
                numerator_quantity: line.quantity.clone(),
                basis_count: recipe.basis_count.clone(),
                unit: line.unit.clone(),
                price,
                cost: None,
            });
        }
        let value = cost_recipe(&RecipeCostInput {
            basis_count: parse_basis_count(&recipe.basis_count).map_err(unavailable)?,
            rows: inputs,
            selling_price: recipe
                .selling_price
                .as_ref()
                .map(|p| Money::parse(p).map_err(unavailable))
                .transpose()?,
        })
        .map_err(validation)?;
        for (row, cost) in rows.iter_mut().zip(value.row_costs) {
            row.cost = cost.map(|m| m.decimal());
        }
        Ok(Some(RecipeCost {
            recipe_version_id: recipe.id,
            basis_count: recipe.basis_count,
            price_mode: "current".into(),
            rows,
            missing_price_line_ids: missing,
            known_subtotal: value.known_subtotal.decimal(),
            total_cost: value.total_cost.map(|m| m.decimal()),
            selling_price: recipe.selling_price,
            cost_ratio: value.cost_ratio,
            ratio_unavailable_reason: value.ratio_unavailable_reason,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    let heads = connection
        .prepare("SELECT material_id,current_version_id FROM recipe_heads ORDER BY material_id")
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(Head {
                material_id: r.get(0)?,
                current_version_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    Ok(SnapshotRows {
        versions: versions(connection, None, None)?,
        lines: lines(connection, None)?,
        heads,
    })
}

pub(super) fn migrate_recipes(connection: &Connection) -> anyhow::Result<()> {
    let journals = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in journals {
        snapshot::validate_source(&payload, seq, 6)?;
    }
    connection.execute_batch("CREATE TABLE recipe_versions(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,copied_from_version_id TEXT REFERENCES recipe_versions(id),basis_count INTEGER NOT NULL CHECK(basis_count BETWEEN 1 AND 1000000000),reference_quantity_micros INTEGER CHECK(reference_quantity_micros>0 AND reference_quantity_micros<=9000000000000000),reference_unit TEXT,reference_input_quantity TEXT,reference_input_unit TEXT,selling_price_micros INTEGER CHECK(selling_price_micros>=0 AND selling_price_micros<=9000000000000000000),cooking_description TEXT NOT NULL,process TEXT NOT NULL,reference_time TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(material_id,previous_version_id) REFERENCES recipe_versions(material_id,id),CHECK((reference_quantity_micros IS NULL AND reference_unit IS NULL AND reference_input_quantity IS NULL AND reference_input_unit IS NULL) OR (reference_quantity_micros IS NOT NULL AND reference_unit IS NOT NULL AND reference_input_quantity IS NOT NULL AND reference_input_unit IS NOT NULL AND basis_count=1)),CHECK(previous_version_id IS NULL OR copied_from_version_id IS NULL));
        CREATE TABLE recipe_lines(id TEXT PRIMARY KEY NOT NULL,recipe_version_id TEXT NOT NULL REFERENCES recipe_versions(id),position INTEGER NOT NULL CHECK(position>=0),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,note TEXT NOT NULL,UNIQUE(recipe_version_id,position),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE recipe_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES recipe_versions(material_id,id));
        CREATE INDEX recipe_versions_material ON recipe_versions(material_id,sequence);
        CREATE INDEX recipe_lines_material ON recipe_lines(material_id,recipe_version_id);
        PRAGMA user_version=7;")?;
    Ok(())
}

pub(super) fn validate_recipe_schema(connection: &Connection) -> anyhow::Result<()> {
    let rows = snapshot_rows(connection)?;
    let materials = catalog::versions(connection, None)?;
    let items = super::read_materials(connection)?;
    let requests = connection
        .prepare("SELECT sequence,operation FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?.to_string(), r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let requests: Vec<_> = requests
        .iter()
        .map(|(seq, operation)| (seq.as_str(), operation.as_str()))
        .collect();
    let sequence = catalog::state(connection)?.revision.parse::<i64>()?;
    validate_snapshot(
        &rows.versions,
        &rows.lines,
        &rows.heads,
        &materials,
        &items,
        &requests,
        sequence,
    )?;
    Ok(())
}

fn copied_content(
    version: &VersionRow,
    lines: &[LineRow],
) -> Result<serde_json::Value, StoreError> {
    let mut components: Vec<_> = lines
        .iter()
        .filter(|l| l.recipe_version_id == version.id)
        .map(|l| &l.line)
        .collect();
    components.sort_by_key(|l| l.position.parse::<usize>().unwrap_or(usize::MAX));
    let components: Vec<_> = components
        .into_iter()
        .map(|l| {
            (
                &l.position,
                &l.material_id,
                &l.material_version_id,
                &l.quantity,
                &l.unit,
                &l.input_quantity,
                &l.input_unit,
                &l.note,
            )
        })
        .collect();
    serde_json::to_value((
        &version.basis_count,
        &version.reference_output,
        &version.selling_price,
        &version.cooking_description,
        &version.process,
        &version.reference_time,
        components,
    ))
    .map_err(unavailable)
}

fn existed_at(material: &MaterialVersion, sequence: i64) -> bool {
    match &material.seq {
        Some(seq) => seq
            .parse::<i64>()
            .is_ok_and(|n| n > 0 && n <= sequence && n.to_string() == *seq),
        None => material.source == "legacy_migration",
    }
}

pub(super) fn validate_snapshot(
    versions: &[VersionRow],
    lines: &[LineRow],
    heads: &[Head],
    materials: &[MaterialVersion],
    items: &[super::Material],
    requests: &[(&str, &str)],
    sequence: i64,
) -> Result<(), StoreError> {
    let bad = || unavailable("레시피 버전·구성·참조 또는 순환 관계가 올바르지 않습니다");
    let by_id: HashMap<_, _> = versions.iter().map(|v| (&v.id, v)).collect();
    let material_versions: HashMap<_, _> = materials.iter().map(|m| (&m.id, m)).collect();
    let material_ids: HashSet<_> = items.iter().map(|m| &m.id).collect();
    let commands: HashMap<_, _> = requests.iter().copied().collect();
    let recipe_commands = requests
        .iter()
        .filter(|(_, operation)| {
            matches!(
                *operation,
                "recipe.create" | "recipe.revise" | "recipe.copy"
            )
        })
        .count();
    if by_id.len() != versions.len() || material_versions.len() != materials.len() {
        return Err(bad());
    }
    if material_ids.len() != items.len()
        || commands.len() != requests.len()
        || recipe_commands != versions.len()
    {
        return Err(bad());
    }
    let mut used_commands = HashSet::new();
    let mut line_ids = HashSet::new();
    let mut grouped = HashMap::<&str, Vec<&RecipeLine>>::new();
    for row in lines {
        Uuid::parse_str(&row.line.id).map_err(unavailable)?;
        if !line_ids.insert(&row.line.id) || !by_id.contains_key(&row.recipe_version_id) {
            return Err(bad());
        }
        grouped
            .entry(&row.recipe_version_id)
            .or_default()
            .push(&row.line);
    }
    let mut histories = BTreeMap::<&str, Vec<&VersionRow>>::new();
    for version in versions {
        for id in [
            &version.id,
            &version.material_id,
            &version.material_version_id,
        ] {
            Uuid::parse_str(id).map_err(unavailable)?;
        }
        let seq = version.seq.parse::<i64>().map_err(unavailable)?;
        let expected_operation = if version.previous_version_id.is_some() {
            "recipe.revise"
        } else if version.copied_from_version_id.is_some() {
            "recipe.copy"
        } else {
            "recipe.create"
        };
        if seq <= 0
            || seq > sequence
            || seq.to_string() != version.seq
            || !used_commands.insert(&version.seq)
            || commands.get(version.seq.as_str()).copied() != Some(expected_operation)
        {
            return Err(bad());
        }
        chrono::DateTime::parse_from_rfc3339(&version.recorded_at).map_err(unavailable)?;
        let owner = material_versions
            .get(&version.material_version_id)
            .ok_or_else(bad)?;
        if owner.material_id != version.material_id
            || owner.name != version.material_name
            || owner.unit != version.material_unit
            || !existed_at(owner, seq)
            || !material_ids.contains(&version.material_id)
        {
            return Err(bad());
        }
        let mut components = grouped.remove(version.id.as_str()).unwrap_or_default();
        components.sort_by_key(|l| l.position.parse::<usize>().unwrap_or(usize::MAX));
        let mut data = RecipeDataInput {
            basis_count: version.basis_count.clone(),
            reference_output: version
                .reference_output
                .as_ref()
                .map(|o| RecipeReferenceInput {
                    quantity: o.input_quantity.clone(),
                    unit: o.input_unit.clone(),
                }),
            selling_price: version.selling_price.clone(),
            cooking_description: version.cooking_description.clone(),
            process: version.process.clone(),
            reference_time: version.reference_time.clone(),
            lines: vec![],
        };
        if let Some(output) = &version.reference_output {
            let unit = Unit::parse(&output.input_unit).map_err(unavailable)?;
            if !unit.same_dimension(&Unit::parse(&owner.unit).map_err(unavailable)?)
                || unit.canonical() != output.unit
                || unit
                    .to_canonical(Quantity::parse(&output.input_quantity).map_err(unavailable)?)
                    .map_err(unavailable)?
                    .decimal()
                    != output.quantity
            {
                return Err(bad());
            }
        }
        for (position, line) in components.iter().enumerate() {
            let material = material_versions
                .get(&line.material_version_id)
                .ok_or_else(bad)?;
            let unit = Unit::parse(&line.input_unit).map_err(unavailable)?;
            if line.position != position.to_string()
                || material.material_id != line.material_id
                || material.name != line.material_name
                || !existed_at(material, seq)
                || !material_ids.contains(&line.material_id)
                || !unit.same_dimension(&Unit::parse(&material.unit).map_err(unavailable)?)
                || unit.canonical() != line.unit
                || unit
                    .to_canonical(Quantity::parse(&line.input_quantity).map_err(unavailable)?)
                    .map_err(unavailable)?
                    .decimal()
                    != line.quantity
            {
                return Err(bad());
            }
            data.lines.push(RecipeLineInput {
                material_id: line.material_id.clone(),
                material_version_id: line.material_version_id.clone(),
                quantity: line.input_quantity.clone(),
                unit: line.input_unit.clone(),
                note: line.note.clone(),
            });
        }
        let original = serde_json::to_value(&data).map_err(unavailable)?;
        normalize_recipe_data(&mut data).map_err(unavailable)?;
        if original != serde_json::to_value(&data).map_err(unavailable)? {
            return Err(bad());
        }
        if let Some(source) = &version.copied_from_version_id {
            let source = by_id.get(source).ok_or_else(bad)?;
            let latest_source = versions
                .iter()
                .filter(|v| v.material_id == source.material_id)
                .filter_map(|v| {
                    v.seq
                        .parse::<i64>()
                        .ok()
                        .filter(|n| *n < seq)
                        .map(|n| (n, &v.id))
                })
                .max_by_key(|(n, _)| *n);
            if source.material_id == version.material_id
                || source.seq.parse::<i64>().map_err(unavailable)? >= seq
                || version.previous_version_id.is_some()
                || copied_content(source, lines)? != copied_content(version, lines)?
                || latest_source.map(|(_, id)| id) != Some(&source.id)
            {
                return Err(bad());
            }
        }
        histories
            .entry(&version.material_id)
            .or_default()
            .push(version);
    }
    if histories.len() != heads.len() {
        return Err(bad());
    }
    let mut head_ids = HashSet::new();
    let mut graph = HashMap::new();
    for head in heads {
        if !head_ids.insert(&head.material_id) {
            return Err(bad());
        }
        let history = histories
            .get_mut(head.material_id.as_str())
            .ok_or_else(bad)?;
        history.sort_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1));
        let mut previous = None;
        let mut previous_seq = 0;
        for version in history.iter() {
            let seq = version.seq.parse::<i64>().map_err(unavailable)?;
            if version.previous_version_id.as_deref() != previous || seq <= previous_seq {
                return Err(bad());
            }
            previous = Some(version.id.as_str());
            previous_seq = seq;
        }
        if previous != Some(head.current_version_id.as_str()) {
            return Err(bad());
        }
        graph.insert(
            head.material_id.clone(),
            lines
                .iter()
                .filter(|l| l.recipe_version_id == head.current_version_id)
                .map(|l| l.line.material_id.clone())
                .collect(),
        );
    }
    ensure_acyclic(&graph).map_err(unavailable)
}
