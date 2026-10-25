use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{bad, seq, tag};
use rusqlite::{Connection, params};

pub(super) fn insert(connection: &Connection, state: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &state.photo_assets {
        let d = &r.descriptor;
        connection.execute("INSERT INTO photo_assets(sha256,asset_key,format,byte_length,width,height,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![d.sha256,d.asset_key,tag(&d.format)?,i64::try_from(d.byte_length).map_err(bad)?,i64::from(d.width),i64::from(d.height),r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.material_profile_versions {
        connection.execute("INSERT INTO material_profile_versions(id,material_id,material_version_id,previous_version_id,copied_from_version_id,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![r.id,r.material_id,r.material_version_id,r.previous_version_id,r.copied_from_version_id,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.material_profile_heads {
        connection
            .execute(
                "INSERT INTO material_profile_heads(material_id,current_version_id) VALUES(?1,?2)",
                params![r.material_id, r.current_version_id],
            )
            .map_err(unavailable)?;
    }
    for r in &state.material_profile_photos {
        let p = &r.photo;
        connection.execute("INSERT INTO material_profile_photos(profile_version_id,id,position,asset_sha256,caption,is_primary) VALUES(?1,?2,?3,?4,?5,?6)",params![r.profile_version_id,p.id,i64::try_from(p.position).map_err(bad)?,p.asset.sha256,p.caption,p.is_primary]).map_err(unavailable)?;
    }
    for r in &state.material_profile_suppliers {
        let s = &r.supplier;
        connection.execute("INSERT INTO material_profile_suppliers(profile_version_id,position,url,description) VALUES(?1,?2,?3,?4)",params![r.profile_version_id,i64::try_from(s.position).map_err(bad)?,s.url,s.description]).map_err(unavailable)?;
    }
    Ok(())
}
