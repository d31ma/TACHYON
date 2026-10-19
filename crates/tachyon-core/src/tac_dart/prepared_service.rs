use super::{Bridge, DartModels, Unit, failure, prepare};
use crate::Failure;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct PreparedService {
    pub(super) class: String,
    pub(super) relative: std::path::PathBuf,
    pub(super) portable: String,
    source: String,
    pub(super) unit: Unit,
}

impl PreparedService {
    pub(super) fn read(snapshot_root: &Path, class: &str, path: &Path) -> Result<Self, Failure> {
        let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
        let portable = relative.to_string_lossy().replace('\\', "/");
        let source = std::fs::read_to_string(path)
            .map_err(|error| failure(&portable, format!("cannot read the service: {error}")))?;
        let unit = prepare(snapshot_root, relative, &source)?;
        Ok(Self {
            class: class.to_owned(),
            relative: relative.to_path_buf(),
            portable,
            source,
            unit,
        })
    }

    pub(super) fn bridge(
        &self,
        lowered: &str,
        dart: &BTreeSet<&str>,
        instances: &BTreeSet<String>,
        models: &DartModels,
    ) -> Result<(String, String), Failure> {
        let (class, portable, unit) = (self.class.as_str(), self.portable.as_str(), &self.unit);
        let bridge = Bridge::read(&unit.source, class, &models.names)
            .and_then(|bridge| bridge.with_stored(&self.source))
            .map_err(|message| failure(portable, message))?;
        bridge
            .check_plan(unit.plan.as_ref())
            .map_err(|message| failure(portable, message))?;
        let held = bridge
            .held_streams(&unit.clients)
            .map_err(|message| failure(portable, message))?;
        let injections = bridge
            .injections(dart, instances, class)
            .map_err(|message| failure(portable, message))?;
        let plain = crate::behavior::ServiceRoles::without_roles(&self.relative, &self.source)?;
        let named = crate::behavior::ProviderQualifiers::read(&self.relative, &plain, class)?;
        let page = Bridge::read(lowered, class, &models.names)
            .map(|page| page.with_sockets(&unit.clients).with_named(class, &named))
            .map_err(|message| failure(portable, message))?;
        let source = page
            .bridge(lowered, class, &injections, &models.names)
            .map_err(|message| failure(portable, message))?;
        let module = bridge.module(class, portable, &injections, unit.plan.as_ref(), held);
        Ok((Bridge::named_for(&source, class), module))
    }
}
