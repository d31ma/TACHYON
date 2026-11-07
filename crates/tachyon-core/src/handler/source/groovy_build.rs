use super::{compiled_digest, compiler_identity};
use crate::groovy_home::GroovyHome;
use std::path::Path;

pub(super) struct GroovyBuild<'build> {
    pub(super) groovy: &'build str,
    pub(super) java: &'build str,
    pub(super) home: &'build GroovyHome,
    pub(super) runtime_jar: &'build Path,
}

impl GroovyBuild<'_> {
    const JAVA: &'static str = "java";

    pub(super) fn digest(&self) -> String {
        let identity = format!(
            "{}{}",
            self.runtime_jar.to_string_lossy(),
            compiler_identity(Self::JAVA)
        );
        compiled_digest(
            self.groovy.as_bytes(),
            self.java.as_bytes(),
            &identity,
            std::env::consts::ARCH,
            std::env::consts::OS,
        )
    }
}
