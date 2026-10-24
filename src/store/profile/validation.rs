use super::*;
use std::collections::{BTreeMap, HashMap};

pub(in crate::store) struct Validation<'a> {
    pub assets: &'a [AssetRow],
    pub versions: &'a [VersionRow],
    pub heads: &'a [Head],
    pub photos: &'a [PhotoRow],
    pub suppliers: &'a [SupplierRow],
    pub materials: &'a [MaterialVersion],
    pub mutations: &'a [(&'a str, &'a str, &'a str, &'a str, &'a str)],
    pub sequence: i64,
}
fn bad() -> StoreError {
    unavailable("사진·구입처 이력과 저장 근거가 일치하지 않습니다")
}
fn uuid(id: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(id).map_err(|_| bad())?.to_string() != id {
        return Err(bad());
    }
    Ok(())
}
fn seq(value: &str, max: i64) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(|_| bad())?;
    if n <= 0 || n > max || n.to_string() != value {
        return Err(bad());
    }
    Ok(n)
}
fn time(value: &str) -> Result<(), StoreError> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(|_| bad())?;
    Ok(())
}
fn same<T: Serialize>(a: &T, b: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(a).map_err(unavailable)?
        == serde_json::to_value(b).map_err(unavailable)?)
}
fn material_seq(v: &MaterialVersion) -> Result<i64, StoreError> {
    match v.seq.as_deref() {
        Some(s) => s.parse().map_err(|_| bad()),
        None if v.source == "legacy_migration" => Ok(0),
        None => Err(bad()),
    }
}

pub(in crate::store) fn validate_snapshot(v: Validation<'_>) -> Result<(), StoreError> {
    let mut assets = HashMap::new();
    for asset in v.assets {
        photo_assets::validate_asset_descriptor(&asset.descriptor).map_err(|_| bad())?;
        seq(&asset.seq, v.sequence)?;
        time(&asset.recorded_at)?;
        if assets
            .insert(asset.descriptor.sha256.as_str(), asset)
            .is_some()
        {
            return Err(bad());
        }
    }
    let mut versions = HashMap::new();
    let mut ordered = BTreeMap::new();
    for row in v.versions {
        for id in [&row.id, &row.material_id, &row.material_version_id] {
            uuid(id)?;
        }
        let n = seq(&row.seq, v.sequence)?;
        time(&row.recorded_at)?;
        if versions.insert(row.id.as_str(), row).is_some()
            || ordered.insert((n, row.material_id.as_str()), row).is_some()
        {
            return Err(bad());
        }
    }
    let mut photos: HashMap<&str, Vec<&ProfilePhoto>> = HashMap::new();
    let mut suppliers: HashMap<&str, Vec<&ProfileSupplier>> = HashMap::new();
    for row in v.photos {
        if !versions.contains_key(row.profile_version_id.as_str()) {
            return Err(bad());
        }
        photos
            .entry(&row.profile_version_id)
            .or_default()
            .push(&row.photo);
    }
    for row in v.suppliers {
        if !versions.contains_key(row.profile_version_id.as_str()) {
            return Err(bad());
        }
        suppliers
            .entry(&row.profile_version_id)
            .or_default()
            .push(&row.supplier);
    }
    for rows in photos.values_mut() {
        rows.sort_by_key(|r| r.position);
    }
    for rows in suppliers.values_mut() {
        rows.sort_by_key(|r| r.position);
    }
    let mut latest: HashMap<&str, &VersionRow> = HashMap::new();
    let mut photo_owners: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut asset_first: HashMap<&str, (i64, &str)> = HashMap::new();
    let mut full_versions: HashMap<&str, MaterialProfileVersion> = HashMap::new();
    let mut used_mutations = HashSet::new();
    for ((n, _), row) in ordered {
        let current_material = v
            .materials
            .iter()
            .filter(|m| m.material_id == row.material_id)
            .map(|m| Ok((material_seq(m)?, m)))
            .collect::<Result<Vec<_>, StoreError>>()?
            .into_iter()
            .filter(|(s, _)| *s <= n)
            .max_by_key(|(s, _)| *s)
            .map(|(_, m)| m)
            .ok_or_else(bad)?;
        if current_material.id != row.material_version_id {
            return Err(bad());
        }
        let previous = latest.get(row.material_id.as_str()).copied();
        if row.previous_version_id.as_deref() != previous.map(|p| p.id.as_str()) {
            return Err(bad());
        }
        if row.previous_version_id.is_some() && row.copied_from_version_id.is_some() {
            return Err(bad());
        }
        let ps = photos.get(row.id.as_str()).cloned().unwrap_or_default();
        let ss = suppliers.get(row.id.as_str()).cloned().unwrap_or_default();
        if ps.len() > MAX_PROFILE_PHOTOS || ss.len() > MAX_PROFILE_SUPPLIERS {
            return Err(bad());
        }
        let mut ids = HashSet::new();
        let mut primary = 0;
        for (position, p) in ps.iter().enumerate() {
            uuid(&p.id)?;
            let mut caption = p.caption.clone();
            text(&mut caption, MAX_PROFILE_CAPTION_CHARS, "사진 설명").map_err(|_| bad())?;
            if caption != p.caption || position != p.position || !ids.insert(&p.id) {
                return Err(bad());
            }
            primary += usize::from(p.is_primary);
            let asset = assets.get(p.asset.sha256.as_str()).ok_or_else(bad)?;
            if asset.descriptor != p.asset || asset.seq.parse::<i64>().map_err(|_| bad())? > n {
                return Err(bad());
            }
            asset_first
                .entry(&p.asset.sha256)
                .or_insert((n, &row.recorded_at));
            if let Some((owner, sha)) = photo_owners.get(p.id.as_str()) {
                if *owner != row.material_id || *sha != p.asset.sha256 {
                    return Err(bad());
                }
            } else {
                photo_owners.insert(&p.id, (&row.material_id, &p.asset.sha256));
            }
        }
        if primary != usize::from(!ps.is_empty()) {
            return Err(bad());
        }
        for (position, s) in ss.iter().enumerate() {
            let mut input = SupplierInput {
                url: s.url.clone(),
                description: s.description.clone(),
            };
            normalize_supplier(&mut input).map_err(|_| bad())?;
            if position != s.position || input.url != s.url || input.description != s.description {
                return Err(bad());
            }
        }
        let full = MaterialProfileVersion {
            id: row.id.clone(),
            material_id: row.material_id.clone(),
            material_version_id: row.material_version_id.clone(),
            previous_version_id: row.previous_version_id.clone(),
            copied_from_version_id: row.copied_from_version_id.clone(),
            photos: ps.into_iter().cloned().collect(),
            suppliers: ss.into_iter().cloned().collect(),
            recorded_at: row.recorded_at.clone(),
            seq: row.seq.clone(),
        };
        let matches = v
            .mutations
            .iter()
            .filter(|m| m.0 == row.seq)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(bad());
        }
        let mutation = matches[0];
        if !used_mutations.insert(mutation.0) {
            return Err(bad());
        }
        if let Some(source) = &row.copied_from_version_id {
            uuid(source)?;
            if mutation.1 != "recipe.copy" || previous.is_some() {
                return Err(bad());
            }
            let source = full_versions.get(source.as_str()).ok_or_else(bad)?;
            if source.material_id == row.material_id
                || latest
                    .get(source.material_id.as_str())
                    .map(|p| p.id.as_str())
                    != Some(source.id.as_str())
            {
                return Err(bad());
            }
            let (source_id, request): (String, crate::recipe::CopyRecipe) =
                serde_json::from_str(mutation.3).map_err(|_| bad())?;
            if request.meta.request_id != mutation.2
                || request.meta.expected_revision != (n - 1).to_string()
            {
                return Err(bad());
            }
            let response: crate::recipe::RecipeMutation =
                serde_json::from_str(mutation.4).map_err(|_| bad())?;
            if source_id != source.material_id
                || response.item.id != row.material_id
                || response.item.version_id != row.material_version_id
                || response.version.seq != row.seq
                || response.version.recorded_at != row.recorded_at
            {
                return Err(bad());
            }
            if full.photos.len() != source.photos.len() || full.suppliers != source.suppliers {
                return Err(bad());
            }
            for (p, s) in full.photos.iter().zip(&source.photos) {
                if p.id == s.id
                    || p.position != s.position
                    || p.asset != s.asset
                    || p.caption != s.caption
                    || p.is_primary != s.is_primary
                {
                    return Err(bad());
                }
            }
        } else {
            if mutation.1 != "profile.revise" {
                return Err(bad());
            }
            let mut request: CanonicalRequest =
                serde_json::from_str(mutation.3).map_err(|_| bad())?;
            if request.request.meta.request_id != mutation.2
                || request.request.meta.expected_revision != (n - 1).to_string()
            {
                return Err(bad());
            }
            let canonical = serde_json::to_string(&request).map_err(unavailable)?;
            normalize(&mut request.request).map_err(|_| bad())?;
            if canonical != serde_json::to_string(&request).map_err(unavailable)?
                || request.material_id != row.material_id
                || request.request.expected_material_version_id != row.material_version_id
                || request.request.expected_profile_version_id != row.previous_version_id
            {
                return Err(bad());
            }
            let response: MaterialProfileDetail =
                serde_json::from_str(mutation.4).map_err(|_| bad())?;
            if response.material.id != row.material_id
                || response.material.version_id != row.material_version_id
                || response.state.revision != row.seq
                || response.state.epoch != request.request.meta.epoch
                || !same(&response.version, &Some(full.clone()))?
            {
                return Err(bad());
            }
            if request.request.photos.len() != full.photos.len()
                || request.request.suppliers.len() != full.suppliers.len()
            {
                return Err(bad());
            }
            let old = previous.and_then(|p| full_versions.get(p.id.as_str()));
            let mut uploads = 0;
            for (input, p) in request.request.photos.iter().zip(&full.photos) {
                if p.is_primary != (request.request.primary_position == Some(p.position)) {
                    return Err(bad());
                }
                match input {
                    ProfilePhotoInput::Existing { photo_id, caption } => {
                        let prior = old
                            .and_then(|v| v.photos.iter().find(|p| &p.id == photo_id))
                            .ok_or_else(bad)?;
                        if p.id != *photo_id || p.caption != *caption || p.asset != prior.asset {
                            return Err(bad());
                        }
                    }
                    ProfilePhotoInput::Upload { caption } => {
                        uploads += 1;
                        if p.caption != *caption
                            || request.upload.as_ref() != Some(&p.asset)
                            || full_versions
                                .values()
                                .any(|v| v.photos.iter().any(|old| old.id == p.id))
                        {
                            return Err(bad());
                        }
                    }
                }
            }
            if uploads != usize::from(request.upload.is_some()) {
                return Err(bad());
            }
            for (input, s) in request.request.suppliers.iter().zip(&full.suppliers) {
                if input.url != s.url || input.description != s.description {
                    return Err(bad());
                }
            }
        }
        latest.insert(&row.material_id, row);
        full_versions.insert(&row.id, full);
    }
    let mut heads = HashSet::new();
    for head in v.heads {
        if !heads.insert(&head.material_id)
            || latest.get(head.material_id.as_str()).map(|v| v.id.as_str())
                != Some(head.current_version_id.as_str())
        {
            return Err(bad());
        }
    }
    if heads.len() != latest.len() || asset_first.len() != assets.len() {
        return Err(bad());
    }
    for (sha, asset) in assets {
        if asset_first.get(sha).copied()
            != Some((
                asset.seq.parse().map_err(|_| bad())?,
                asset.recorded_at.as_str(),
            ))
        {
            return Err(bad());
        }
    }
    for mutation in v.mutations {
        if mutation.1 == "profile.revise" && !used_mutations.contains(mutation.0) {
            return Err(bad());
        }
        if mutation.1 == "recipe.copy" {
            let (source, _): (String, crate::recipe::CopyRecipe) =
                serde_json::from_str(mutation.3).map_err(|_| bad())?;
            let n = seq(mutation.0, v.sequence)?;
            let had_profile = v
                .versions
                .iter()
                .any(|r| r.material_id == source && r.seq.parse::<i64>().is_ok_and(|s| s < n));
            if had_profile && !used_mutations.contains(mutation.0) {
                return Err(bad());
            }
        }
    }
    Ok(())
}
