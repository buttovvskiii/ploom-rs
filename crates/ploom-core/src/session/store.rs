//! Persistence in the session's data directory.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use super::limit::LimitRecord;
use crate::model::CustomProfile;

pub(crate) const FILE_NAME: &str = "session.json";

const VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Persisted {
    version: u32,
    last_device_id: Option<String>,
    /// Device id → custom profile currently flashed onto it.
    custom_profiles: BTreeMap<String, CustomProfile>,
    /// Device id → stick limit settings and progress.
    limits: BTreeMap<String, LimitRecord>,
}

#[derive(Debug)]
pub(crate) struct Store {
    path: PathBuf,
    data: Persisted,
}

impl Store {
    pub(crate) fn load(dir: &Path) -> Self {
        let path = dir.join(FILE_NAME);
        let data = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                warn!(path = %path.display(), error = %e, "ignoring unreadable session file");
                Persisted::default()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Persisted::default(),
            Err(e) => {
                warn!(path = %path.display(), error = %e, "cannot read session file");
                Persisted::default()
            }
        };
        Self { path, data }
    }

    pub(crate) fn last_device_id(&self) -> Option<String> {
        self.data.last_device_id.clone()
    }

    pub(crate) fn custom_profile(&self, device_id: &str) -> Option<CustomProfile> {
        self.data.custom_profiles.get(device_id).copied()
    }

    pub(crate) fn limit(&self, device_id: &str) -> Option<LimitRecord> {
        self.data.limits.get(device_id).copied()
    }

    pub(crate) fn set_limit(&mut self, device_id: &str, record: LimitRecord) -> io::Result<()> {
        if self.data.limits.get(device_id) == Some(&record) {
            return Ok(());
        }
        self.data.limits.insert(device_id.to_owned(), record);
        self.save()
    }

    pub(crate) fn set_last_device_id(&mut self, id: &str) -> io::Result<()> {
        if self.data.last_device_id.as_deref() == Some(id) {
            return Ok(());
        }
        self.data.last_device_id = Some(id.to_owned());
        self.save()
    }

    pub(crate) fn set_custom_profile(
        &mut self,
        device_id: &str,
        profile: Option<CustomProfile>,
    ) -> io::Result<()> {
        let changed = match profile {
            Some(p) => self.data.custom_profiles.insert(device_id.to_owned(), p) != Some(p),
            None => self.data.custom_profiles.remove(device_id).is_some(),
        };
        if changed { self.save() } else { Ok(()) }
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        self.data.version = VERSION;
        let json = serde_json::to_vec_pretty(&self.data).map_err(io::Error::other)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use ploom_protocol::HeatingMode;

    use super::*;

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let profile = CustomProfile {
            base: HeatingMode::Delicate,
            hold_temp_c: 255,
            hold_secs: 200,
        };
        {
            let mut s = Store::load(dir.path());
            assert_eq!(s.last_device_id(), None);
            s.set_last_device_id("dev-1").unwrap();
            s.set_custom_profile("dev-1", Some(profile)).unwrap();
        }
        let mut s = Store::load(dir.path());
        assert_eq!(s.last_device_id().as_deref(), Some("dev-1"));
        assert_eq!(s.custom_profile("dev-1"), Some(profile));
        assert_eq!(s.custom_profile("dev-2"), None);
        s.set_custom_profile("dev-1", None).unwrap();
        assert_eq!(Store::load(dir.path()).custom_profile("dev-1"), None);

        let text = std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert!(text.contains("\"lastDeviceId\": \"dev-1\""), "{text}");
    }

    #[test]
    fn limit_round_trip_and_old_files() {
        let dir = tempfile::tempdir().unwrap();
        // A file written before the limit existed still loads.
        std::fs::write(
            dir.path().join(FILE_NAME),
            br#"{"version":1,"lastDeviceId":"dev-1","customProfiles":{}}"#,
        )
        .unwrap();
        let mut s = Store::load(dir.path());
        assert_eq!(s.last_device_id().as_deref(), Some("dev-1"));
        assert_eq!(s.limit("dev-1"), None);
        let record = LimitRecord {
            sticks_used: 4,
            locked_until_ms: Some(123),
            locked_by_limit: true,
            ..LimitRecord::default()
        };
        s.set_limit("dev-1", record).unwrap();
        assert_eq!(Store::load(dir.path()).limit("dev-1"), Some(record));
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), b"{not json").unwrap();
        let mut s = Store::load(dir.path());
        assert_eq!(s.last_device_id(), None);
        s.set_last_device_id("x").unwrap();
        assert_eq!(
            Store::load(dir.path()).last_device_id().as_deref(),
            Some("x")
        );
    }

    #[test]
    fn creates_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        Store::load(&nested).set_last_device_id("x").unwrap();
        assert!(nested.join(FILE_NAME).exists());
    }
}
