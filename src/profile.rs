use crate::{
    business::BusinessMeta,
    catalog::{CatalogState, MaterialSummary},
    photo_assets::PhotoAssetDescriptor,
};
use serde::{Deserialize, Serialize};

pub const MAX_PROFILE_PHOTOS: usize = 20;
pub const MAX_PROFILE_SUPPLIERS: usize = 20;
pub const MAX_PROFILE_CAPTION_CHARS: usize = 500;
pub const MAX_SUPPLIER_URL_CHARS: usize = 2048;
pub const MAX_SUPPLIER_DESCRIPTION_CHARS: usize = 2000;
pub const MAX_PROFILE_METADATA_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviseMaterialProfile {
    pub meta: BusinessMeta,
    pub expected_material_version_id: String,
    pub expected_profile_version_id: Option<String>,
    pub photos: Vec<ProfilePhotoInput>,
    pub primary_position: Option<usize>,
    pub suppliers: Vec<SupplierInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfilePhotoInput {
    Existing { photo_id: String, caption: String },
    Upload { caption: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplierInput {
    pub url: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePhoto {
    pub id: String,
    pub position: usize,
    pub asset: PhotoAssetDescriptor,
    pub caption: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSupplier {
    pub position: usize,
    pub url: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialProfileVersion {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub photos: Vec<ProfilePhoto>,
    pub suppliers: Vec<ProfileSupplier>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialProfileDetail {
    pub material: MaterialSummary,
    pub version: Option<MaterialProfileVersion>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialProfileVersionSummary {
    pub id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub photo_count: usize,
    pub supplier_count: usize,
    pub primary_photo_id: Option<String>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialProfileHistory {
    pub material_id: String,
    pub versions: Vec<MaterialProfileVersionSummary>,
    pub state: CatalogState,
}
