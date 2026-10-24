use crate::{BuildOptions, BuildResult, DevelopmentOrigin, Failure, Project, WebCompiler};

#[derive(Clone, Debug)]
pub(in crate::server) struct SiteBuilder {
    options: BuildOptions,
    native: Option<DevelopmentOrigin>,
}

impl SiteBuilder {
    pub(in crate::server) const fn new(
        options: BuildOptions,
        native: Option<DevelopmentOrigin>,
    ) -> Self {
        Self { options, native }
    }

    pub(in crate::server) async fn build(&self, project: &Project) -> Result<BuildResult, Failure> {
        let Some(development) = &self.native else {
            return WebCompiler::build_project_async(project, &self.options).await;
        };
        development.declare_in(project.snapshot_root())?;
        WebCompiler::build_project_for_native(project, &self.options, development.target()).await
    }
}
