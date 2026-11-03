use super::config::NativeApplication;
use super::host::{native_io, write};
use super::linux_app_image_runtime::LinuxAppImageRuntime;
use super::linux_package_files::LinuxPackageFiles;
use super::linux_package_names::LinuxPackageNames;
use super::linux_package_sources::LinuxPackageSources;
use super::native_reproducibility::NativeReproducibility;
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

pub(super) struct LinuxPackages;

impl LinuxPackages {
    const DEB_ARGUMENTS: [&'static str; 3] = ["--root-owner-group", "-Zxz", "--build"];
    const SQUASHFS_ARGUMENTS: [&'static str; 6] = [
        "-root-owned",
        "-noappend",
        "-no-xattrs",
        "-comp",
        "zstd",
        "-quiet",
    ];
    const APP_DIR: &'static str = "AppDir";
    const DEB_ROOT: &'static str = "deb";
    const SQUASHFS_IMAGE: &'static str = "AppDir.squashfs";

    pub(super) fn check(
        application: &NativeApplication,
        target: NativeTarget,
        release: bool,
    ) -> Result<(), Failure> {
        let packaged = [target == NativeTarget::Linux, release] == [true, true];
        packaged
            .then(|| LinuxPackageNames::of(application, std::env::consts::ARCH))
            .transpose()
            .map(|_| ())
    }

    pub(super) async fn published(
        application: &NativeApplication,
        release: bool,
        web_bundle: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        Self::published_when(application, release, web_bundle, stage).await
    }

    async fn published_when(
        application: &NativeApplication,
        release: bool,
        web_bundle: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        if !release {
            return Ok(PathBuf::from(&application.executable_name));
        }
        Self::build(application, web_bundle, stage).await
    }

    async fn build(
        application: &NativeApplication,
        web_bundle: &Path,
        stage: &Path,
    ) -> Result<PathBuf, Failure> {
        let names = LinuxPackageNames::of(application, std::env::consts::ARCH)?;
        let work = native_io(tempfile::tempdir_in(stage), stage)?;
        let bundle = stage.join(&application.executable_name);
        let sources = LinuxPackageSources {
            application,
            names: &names,
            bundle: &bundle,
            web_bundle,
        };
        let deb_root = work.path().join(Self::DEB_ROOT);
        Self::install(&sources, &deb_root).await?;
        Self::build_deb(&sources, &deb_root, &stage.join(names.deb_file())).await?;
        let app_dir = work.path().join(Self::APP_DIR);
        Self::install(&sources, &app_dir).await?;
        Self::app_dir_entries(&sources, &app_dir).await?;
        let image = work.path().join(Self::SQUASHFS_IMAGE);
        Self::squash(&app_dir, &image).await?;
        let app_image = names.app_image_file(application);
        Self::join_runtime(&names, &image, &stage.join(&app_image)).await?;
        Ok(PathBuf::from(app_image))
    }

    async fn install(sources: &LinuxPackageSources<'_>, root: &Path) -> Result<(), Failure> {
        let names = sources.names;
        let folder = root.join(names.installed_folder());
        let lib = folder.parent().unwrap_or(root);
        native_io(std::fs::create_dir_all(lib), lib)?;
        let bundle = Self::text(sources.bundle);
        let installed = Self::text(&folder);
        NativeReproducibility::run_long_tool("cp", &["-R", "-p", &bundle, &installed]).await?;
        let command = format!(
            "../lib/{}/bin/{}",
            names.package, sources.application.executable_name
        );
        Self::link(&command, &root.join("usr/bin").join(&names.package)).await?;
        let share = root.join("usr/share");
        let desktop = LinuxPackageFiles::desktop_entry(sources.application, names);
        let desktop_file = format!("applications/{}.desktop", names.desktop_id);
        write(&share.join(desktop_file), desktop.as_bytes())?;
        let epoch = NativeReproducibility::source_date_epoch();
        let metainfo = LinuxPackageFiles::metainfo(sources.application, names, epoch);
        let metainfo_file = format!("metainfo/{}.metainfo.xml", names.desktop_id);
        write(&share.join(metainfo_file), metainfo.as_bytes())?;
        Self::install_icons(sources, &share.join("icons/hicolor"))
    }

    fn install_icons(sources: &LinuxPackageSources<'_>, theme: &Path) -> Result<(), Failure> {
        for (edge, source) in &sources.application.icons {
            let file = format!("{edge}x{edge}/apps/{}.png", sources.names.desktop_id);
            Self::copy_icon(sources.web_bundle, source, &theme.join(file))?;
        }
        Ok(())
    }

    fn copy_icon(web_bundle: &Path, source: &str, destination: &Path) -> Result<(), Failure> {
        let origin = web_bundle.join(source.trim_start_matches('/'));
        if !origin.is_file() {
            return Ok(());
        }
        let folder = destination.parent().unwrap_or(destination);
        native_io(std::fs::create_dir_all(folder), folder)?;
        native_io(std::fs::copy(&origin, destination).map(|_| ()), destination)
    }

    async fn build_deb(
        sources: &LinuxPackageSources<'_>,
        root: &Path,
        deb: &Path,
    ) -> Result<(), Failure> {
        let maintainer = std::env::var(LinuxPackageFiles::MAINTAINER).ok();
        let control =
            LinuxPackageFiles::control(sources.application, sources.names, maintainer.as_deref());
        write(&root.join("DEBIAN/control"), control.as_bytes())?;
        let root = Self::text(root);
        let deb = Self::text(deb);
        let mut arguments = Self::DEB_ARGUMENTS.to_vec();
        arguments.extend([root.as_str(), deb.as_str()]);
        NativeReproducibility::run_long_tool("dpkg-deb", &arguments).await?;
        Ok(())
    }

    async fn app_dir_entries(
        sources: &LinuxPackageSources<'_>,
        app_dir: &Path,
    ) -> Result<(), Failure> {
        let names = sources.names;
        let command = format!("usr/bin/{}", names.package);
        Self::link(&command, &app_dir.join("AppRun")).await?;
        let desktop = LinuxPackageFiles::desktop_entry(sources.application, names);
        let desktop_file = format!("{}.desktop", names.desktop_id);
        write(&app_dir.join(desktop_file), desktop.as_bytes())?;
        let icon = sources.application.largest_icon().unwrap_or_default();
        let icon_file = app_dir.join(format!("{}.png", names.desktop_id));
        Self::copy_icon(sources.web_bundle, icon, &icon_file)?;
        Self::copy_icon(sources.web_bundle, icon, &app_dir.join(".DirIcon"))
    }

    async fn squash(app_dir: &Path, image: &Path) -> Result<(), Failure> {
        let app_dir = Self::text(app_dir);
        let image = Self::text(image);
        let mut arguments = vec![app_dir.as_str(), image.as_str()];
        arguments.extend(Self::SQUASHFS_ARGUMENTS);
        NativeReproducibility::run_long_tool("mksquashfs", &arguments).await?;
        Ok(())
    }

    async fn join_runtime(
        names: &LinuxPackageNames,
        image: &Path,
        app_image: &Path,
    ) -> Result<(), Failure> {
        let mut bytes = LinuxAppImageRuntime::bytes(names.machine).await?;
        bytes.extend(native_io(std::fs::read(image), image)?);
        write(app_image, &bytes)?;
        NativeReproducibility::run_tool("chmod", &["755", &Self::text(app_image)]).await?;
        Ok(())
    }

    async fn link(target: &str, link: &Path) -> Result<(), Failure> {
        let folder = link.parent().unwrap_or(link);
        native_io(std::fs::create_dir_all(folder), folder)?;
        NativeReproducibility::run_tool("ln", &["-s", target, &Self::text(link)]).await?;
        Ok(())
    }

    fn text(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }
}
