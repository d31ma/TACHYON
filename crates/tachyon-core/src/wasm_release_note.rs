pub(crate) struct WasmReleaseNote {
    pub(crate) optimized: &'static str,
    pub(crate) unoptimized: &'static str,
}

impl WasmReleaseNote {
    pub(crate) fn for_build(&self, profile: crate::BuildProfile, shrunk: bool) -> Option<String> {
        self.text(profile, shrunk).map(String::from)
    }

    const fn text(&self, profile: crate::BuildProfile, shrunk: bool) -> Option<&'static str> {
        match (profile, shrunk) {
            (crate::BuildProfile::Development, _) => None,
            (crate::BuildProfile::Release, true) => Some(self.optimized),
            (crate::BuildProfile::Release, false) => Some(self.unoptimized),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WasmReleaseNote;
    use crate::BuildProfile;

    const NOTE: WasmReleaseNote = WasmReleaseNote {
        optimized: "small",
        unoptimized: "large",
    };

    #[test]
    fn only_a_release_build_explains_its_compiler() {
        assert_eq!(NOTE.for_build(BuildProfile::Development, true), None);
        assert_eq!(
            NOTE.for_build(BuildProfile::Release, true).as_deref(),
            Some("small")
        );
        assert_eq!(
            NOTE.for_build(BuildProfile::Release, false).as_deref(),
            Some("large")
        );
    }
}
