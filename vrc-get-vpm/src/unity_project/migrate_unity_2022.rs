use crate::unity_project::{AddPackageErr, AddPackageOperation};
use crate::{PackageCollection, UnityProject, VersionSelector};
use crate::{PackageInstaller, VRCHAT_RECOMMENDED_2022_UNITY, io};
use log::warn;

#[non_exhaustive]
#[derive(Debug)]
pub enum MigrateUnity2022Error {
    UnityVersionMismatch,
    VpmPackageNotFound(&'static str),
    AddPackageErr(AddPackageErr),
    Io(io::Error),
}

impl std::error::Error for MigrateUnity2022Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrateUnity2022Error::AddPackageErr(err) => Some(err),
            MigrateUnity2022Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for MigrateUnity2022Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateUnity2022Error::UnityVersionMismatch => write!(f, "Unity version is not 2019.x"),
            MigrateUnity2022Error::VpmPackageNotFound(name) => {
                write!(f, "VPM package {} not found", name)
            }
            MigrateUnity2022Error::AddPackageErr(err) => write!(f, "{}", err),
            MigrateUnity2022Error::Io(err) => write!(f, "{}", err),
        }
    }
}

impl From<AddPackageErr> for MigrateUnity2022Error {
    fn from(err: AddPackageErr) -> Self {
        MigrateUnity2022Error::AddPackageErr(err)
    }
}

impl From<io::Error> for MigrateUnity2022Error {
    fn from(err: io::Error) -> Self {
        MigrateUnity2022Error::Io(err)
    }
}

type Result<T = (), E = MigrateUnity2022Error> = std::result::Result<T, E>;

impl UnityProject {
    pub async fn migrate_unity_2022(
        &mut self,
        collection: &impl PackageCollection,
        installer: &impl PackageInstaller,
    ) -> Result {
        migrate_unity_2022(self, collection, installer).await
    }
}

async fn migrate_unity_2022(
    project: &mut UnityProject,
    collection: &impl PackageCollection,
    installer: &impl PackageInstaller,
) -> Result {
    // See https://misskey.niri.la/notes/9nod7sk4sr for migration process
    if project.unity_version().major() != 2019 {
        return Err(MigrateUnity2022Error::UnityVersionMismatch);
    }

    // since this command is made for projects with VPM VRCSDK, wan if not
    if !is_vpm_vrcsdk_installed(project) {
        warn!("It looks migrating projects without vpm VRCSDK. this may not intended");
    }

    // remove legacy XR packages
    project
        .upm_manifest
        .remove_dependency("com.unity.xr.oculus.standalone");
    project
        .upm_manifest
        .remove_dependency("com.unity.xr.openvr.standalone");

    // upgrade VRCSDK if installed
    let mut packages = vec![];
    let migrating_packages = [
        "com.vrchat.base",
        "com.vrchat.avatars",
        "com.vrchat.worlds",
        "com.vrchat.core.vpm-resolver",
    ];
    for package in migrating_packages {
        if project.get_locked(package).is_some() {
            let unity_version = Some(VRCHAT_RECOMMENDED_2022_UNITY);
            let Some(vrcsdk) = collection
                .find_package_by_name(package, VersionSelector::latest_for(unity_version, false))
            else {
                return Err(MigrateUnity2022Error::VpmPackageNotFound(package));
            };
            packages.push(vrcsdk);
        }
    }

    if !packages.is_empty() {
        // install packages
        let request = project
            .add_package_request(
                collection,
                &packages,
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await?;
        project.apply_pending_changes(installer, request).await?;
    }

    Ok(())
}

// memo /Applications/Unity/Hub/Editor/2022.3.6f1/Unity.app/Contents/MacOS/Unity -quit -batchmode -projectPath .
fn is_vpm_vrcsdk_installed(project: &UnityProject) -> bool {
    if project.get_locked("com.vrchat.base").is_some()
        || project.get_locked("com.vrchat.avatars").is_some()
        || project.get_locked("com.vrchat.worlds").is_some()
    {
        // VRCSDK is installed
        return true;
    }
    if project.get_locked("com.vrchat.core.vpm-resolver").is_some() {
        // VPM Resolver is installed so It looks it's a vpm project.
        return true;
    }
    // otherwice warn
    false
}
