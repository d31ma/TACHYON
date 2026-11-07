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
        Self::build_for(project, &self.options, self.native.as_ref()).await
    }

    async fn build_for(
        project: &Project,
        options: &BuildOptions,
        native: Option<&DevelopmentOrigin>,
    ) -> Result<BuildResult, Failure> {
        match native {
            None => WebCompiler::build_project_async(project, options).await,
            Some(development) => {
                development.declare_in(project.snapshot_root())?;
                WebCompiler::build_project_for_native(project, options, development.target()).await
            }
        }
    }
}
