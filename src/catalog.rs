use crate::domain::ResolvedPrice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogState {
    pub epoch: String,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationMeta {
    pub request_id: String,
    pub epoch: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveFilter {
    #[default]
    Active,
    Archived,
    All,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CatalogFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub archive: ArchiveFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSummary {
    pub id: String,
    pub version_id: String,
    pub name: String,
    pub unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub kind: String,
    pub description: String,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub price: ResolvedPrice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialVersion {
    pub id: String,
    pub material_id: String,
    pub name: String,
    pub unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub kind: String,
    pub description: String,
    pub archived: bool,
    pub recorded_at: String,
    pub seq: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPriceVersion {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub quantity: String,
    pub amount: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDetail {
    #[serde(flatten)]
    pub item: MaterialSummary,
    pub versions: Vec<MaterialVersion>,
    pub prices: Vec<MaterialPriceVersion>,
    #[serde(flatten)]
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMutation<T> {
    #[serde(flatten)]
    pub value: T,
    #[serde(flatten)]
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCatalogMaterial {
    #[serde(flatten)]
    pub meta: MutationMeta,
    pub name: String,
    pub unit: String,
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseCatalogMaterial {
    #[serde(flatten)]
    pub meta: MutationMeta,
    pub expected_version_id: String,
    pub name: String,
    pub unit: String,
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveCatalogMaterial {
    #[serde(flatten)]
    pub meta: MutationMeta,
    pub expected_version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCatalogPrice {
    #[serde(flatten)]
    pub meta: MutationMeta,
    pub expected_version_id: String,
    pub quantity: String,
    pub amount: String,
    pub unit: String,
}
