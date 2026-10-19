use super::config::NativeApplication;
use super::host::{native_io, run_tool, write};
use super::windows_architecture::WindowsArchitecture;
use super::windows_package_manifest::WindowsPackageManifest;
use super::windows_package_tiles::WindowsPackageTiles;
use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) struct WindowsRelease {
    certificate: Option<PathBuf>,
    password: Option<String>,
    thumbprint: Option<String>,
    publisher: String,
    timestamp: Option<String>,
}

impl WindowsRelease {
    pub(super) const CERTIFICATE: &'static str = "TAC_WINDOWS_CERTIFICATE";
    pub(super) const CERTIFICATE_PASSWORD: &'static str = "TAC_WINDOWS_CERTIFICATE_PASSWORD";
    pub(super) const CERTIFICATE_THUMBPRINT: &'static str = "TAC_WINDOWS_CERTIFICATE_THUMBPRINT";
    pub(super) const CERTIFICATE_PUBLISHER: &'static str = "TAC_WINDOWS_CERTIFICATE_PUBLISHER";
    pub(super) const CERTIFICATE_TIMESTAMP_URL: &'static str =
        "TAC_WINDOWS_CERTIFICATE_TIMESTAMP_URL";
    pub(super) const PACKER: &'static str = "makeappx.exe";
    pub(super) const SIGNER: &'static str = "signtool.exe";
    const THUMBPRINT_LENGTH: usize = 40;
    const HELP: &'static str = "Set TAC_WINDOWS_CERTIFICATE to a .pfx file (with \
         TAC_WINDOWS_CERTIFICATE_PASSWORD when it has one), or TAC_WINDOWS_CERTIFICATE_THUMBPRINT to \
         the SHA-1 thumbprint of a certificate in your certificate store. Set \
         TAC_WINDOWS_CERTIFICATE_PUBLISHER to the certificate's subject, such as \
         CN=Contoso Ltd, O=Contoso Ltd, C=US. To build the app folder without a package, remove \
         --release.";

    pub(super) fn from_environment(
        lookup: impl Fn(&str) -> Option<OsString>,
        working_directory: &Path,
    ) -> Result<Self, Failure> {
        let text = |name: &str| {
            lookup(name)
                .map(|value| value.to_string_lossy().trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let publisher = Self::require_publisher(text(Self::CERTIFICATE_PUBLISHER))?;
        let certificate = text(Self::CERTIFICATE).map(|named| working_directory.join(named));
        let thumbprint = text(Self::CERTIFICATE_THUMBPRINT)
            .map(|value| value.replace(' ', "").to_ascii_uppercase());
        Self::require_one_signer(certificate.as_deref(), thumbprint.as_deref())?;
        let timestamp = text(Self::CERTIFICATE_TIMESTAMP_URL);
        Self::require_timestamp(timestamp.as_deref())?;
        Ok(Self {
            certificate,
            password: text(Self::CERTIFICATE_PASSWORD),
            thumbprint,
            publisher,
            timestamp,
        })
    }

    fn require_publisher(publisher: Option<String>) -> Result<String, Failure> {
        let missing = format!(
            "A release Windows build is an MSIX package whose publisher is the signing certificate's \
             subject, but {} is missing or has no CN= part.",
            Self::CERTIFICATE_PUBLISHER
        );
        publisher
            .filter(|subject| subject.contains("CN="))
            .ok_or_else(|| Self::failure(&missing))
    }

    fn require_one_signer(
        certificate: Option<&Path>,
        thumbprint: Option<&str>,
    ) -> Result<(), Failure> {
        match (certificate, thumbprint) {
            (Some(_), Some(_)) => Err(Self::failure(&format!(
                "Set only one of {} and {}: the package is signed once.",
                Self::CERTIFICATE,
                Self::CERTIFICATE_THUMBPRINT
            ))),
            (None, None) => Err(Self::failure(&format!(
                "A release Windows build is signed with a certificate from the environment, but \
                 neither {} nor {} is set.",
                Self::CERTIFICATE,
                Self::CERTIFICATE_THUMBPRINT
            ))),
            (Some(file), None) => file.is_file().then_some(()).ok_or_else(|| {
                Self::failure(&format!(
                    "{} names '{}', which is not a certificate file.",
                    Self::CERTIFICATE,
                    file.display()
                ))
            }),
            (None, Some(thumbprint)) => {
                let hexadecimal = thumbprint.bytes().all(|byte| byte.is_ascii_hexdigit());
                [hexadecimal, thumbprint.len() == Self::THUMBPRINT_LENGTH]
                    .iter()
                    .all(|fits| *fits)
                    .then_some(())
                    .ok_or_else(|| {
                        Self::failure(&format!(
                            "{} must be the 40 hexadecimal digits of a SHA-1 certificate thumbprint.",
                            Self::CERTIFICATE_THUMBPRINT
                        ))
                    })
            }
        }
    }

    fn require_timestamp(timestamp: Option<&str>) -> Result<(), Failure> {
        let web = |url: &str| {
            ["http://", "https://"]
                .iter()
                .any(|scheme| url.starts_with(scheme))
        };
        timestamp.is_none_or(web).then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "{} must be an http:// or https:// timestamp server.",
                Self::CERTIFICATE_TIMESTAMP_URL
            ))
        })
    }

    #[cfg(test)]
    pub(super) fn publisher(&self) -> &str {
        &self.publisher
    }

    pub(super) fn pack_arguments(layout: &str, package: &str) -> Vec<String> {
        ["pack", "/o", "/h", "SHA256", "/d", layout, "/p", package]
            .map(String::from)
            .to_vec()
    }

    pub(super) fn sign_arguments(&self, package: &str) -> Vec<String> {
        let file = self
            .certificate
            .iter()
            .flat_map(|certificate| [String::from("/f"), certificate.display().to_string()]);
        let password = self
            .certificate
            .as_ref()
            .and(self.password.as_ref())
            .into_iter()
            .flat_map(|password| [String::from("/p"), password.clone()]);
        let store = self
            .thumbprint
            .iter()
            .flat_map(|thumbprint| [String::from("/sha1"), thumbprint.clone()]);
        let timestamp = self.timestamp.iter().flat_map(|url| {
            [
                String::from("/tr"),
                url.clone(),
                String::from("/td"),
                String::from("SHA256"),
            ]
        });
        ["sign", "/fd", "SHA256"]
            .map(String::from)
            .into_iter()
            .chain(file)
            .chain(password)
            .chain(store)
            .chain(timestamp)
            .chain(std::iter::once(String::from(package)))
            .collect()
    }

    pub(super) fn stage_layout(
        &self,
        application: &NativeApplication,
        web_bundle: &Path,
        layout: &Path,
        architecture: WindowsArchitecture,
    ) -> Result<(), Failure> {
        let manifest = WindowsPackageManifest {
            application,
            publisher: &self.publisher,
            architecture,
        };
        write(
            &layout.join(WindowsPackageManifest::FILE),
            manifest.xml()?.as_bytes(),
        )?;
        WindowsPackageTiles::stage_all(application, web_bundle, layout)
    }

    pub(super) async fn published(
        release: Option<Self>,
        application: &NativeApplication,
        bundle: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        Self::published_path(release, application, bundle, stage).await
    }

    async fn published_path(
        release: Option<Self>,
        application: &NativeApplication,
        bundle: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        match release {
            Some(release) => release.package(application, bundle, stage).await,
            None => Ok(PathBuf::from(&application.executable_name)),
        }
    }

    async fn package(
        &self,
        application: &NativeApplication,
        layout: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        let file = format!("{}.msix", application.executable_name);
        let built = stage.join(&file);
        let layout_text = Self::tool_path(layout)?;
        let built_text = Self::tool_path(&built)?;
        Self::run(
            Self::PACKER,
            &Self::pack_arguments(&layout_text, &built_text),
        )
        .await?;
        Self::run(Self::SIGNER, &self.sign_arguments(&built_text)).await?;
        let published = layout.join(&file);
        native_io(std::fs::rename(&built, &published), &published)?;
        Ok(PathBuf::from(&application.executable_name).join(file))
    }

    async fn run(program: &str, arguments: &[String]) -> Result<String, Failure> {
        let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
        run_tool(program, &borrowed).await
    }

    fn tool_path(path: &Path) -> Result<String, Failure> {
        path.to_str()
            .map(|text| text.trim_start_matches(r"\\?\").to_owned())
            .ok_or_else(|| Self::failure("The Windows package path is not valid Unicode."))
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}
