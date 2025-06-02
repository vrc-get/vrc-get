use async_compression::futures::bufread::GzipDecoder;
use fs_extra::error::ErrorKind;
use futures::io::BufReader;
use futures::*;
use indexmap::IndexMap;
use indexmap::map::Entry;
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::mem::forget;
use std::path::{Path, PathBuf};
use std::{fmt, io};
use tokio_util::compat::*;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo, DirEntry, IoTrait};

use crate::utils::PathExt;
use crate::utils::TarArchive;
pub use alcom_template::*;
use vrc_get_vpm::UnityProject;
use vrc_get_vpm::version::{DependencyRange, UnityVersion, VersionRange};

pub mod alcom_template;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

const AVATARS_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.vrchat.avatars";
const WORLDS_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.vrchat.worlds";
const BLANK_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.blank";
const UNITY_2019_4_31: UnityVersion = UnityVersion::new_f1(2019, 4, 31);
const UNITY_2022_3_6: UnityVersion = UnityVersion::new_f1(2022, 3, 6);
const UNITY_2022_3_22: UnityVersion = UnityVersion::new_f1(2022, 3, 22);
const VRCHAT_UNITY_VERSIONS: &[UnityVersion] = &[
    UnityVersion::new_f1(2019, 4, 31),
    UnityVersion::new_f1(2022, 3, 6),
    UnityVersion::new_f1(2022, 3, 22),
];
const VCC_TEMPLATE_PREFIX: &str = "com.anatawa12.vrc-get.vcc.";
const UNNAMED_TEMPLATE_PREFIX: &str = "com.anatawa12.vrc-get.user.";

pub struct ProjectTemplateInfo {
    pub display_name: String,
    pub id: String,
    pub unity_versions: Vec<UnityVersion>,
    pub alcom_template: Option<AlcomTemplate>,
    pub source_path: Option<PathBuf>,
    // If the base template does not exist, the template is not available.
    pub available: bool,
}

#[allow(dead_code)]
pub async fn load_resolve_all_templates(
    io: &DefaultEnvironmentIo,
    unity_versions: &[UnityVersion],
) -> io::Result<Vec<ProjectTemplateInfo>> {
    let (alcom, vcc) = join!(
        load_resolve_alcom_templates(io, unity_versions),
        load_vcc_templates(io)
    );
    Ok(alcom.into_iter().chain(vcc.into_iter()).collect())
}

pub async fn load_vcc_templates(io: &DefaultEnvironmentIo) -> Vec<ProjectTemplateInfo> {
    let mut templates = Vec::new();

    let path = io.resolve("Templates".as_ref());
    let mut dir = match io.read_dir("Templates".as_ref()).await {
        Ok(dir) => dir,
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warn!("failed to read vrc-get/templates directory {path:?}: {e}");
            return Vec::new();
        }
    };
    while let Ok(Some(dir)) = dir.try_next().await {
        if !dir.file_type().await.map(|x| x.is_dir()).unwrap_or(false) {
            continue;
        }

        let Ok(name) = dir.file_name().into_string() else {
            continue;
        };

        let path = path.join(&name);

        // check package.json
        let Ok(pkg_json) = tokio::fs::metadata(path.join("package.json")).await else {
            continue;
        };
        if !pkg_json.is_file() {
            continue;
        }

        match UnityProject::load(DefaultProjectIo::new(path.as_path().into())).await {
            Err(e) => {
                warn!("failed to load user template {name}: {e}");
            }
            Ok(p) => templates.push(ProjectTemplateInfo {
                display_name: name.clone(),
                id: format!("{}{}", VCC_TEMPLATE_PREFIX, name),
                unity_versions: vec![p.unity_version()],
                alcom_template: None,
                source_path: Some(path),
                available: true,
            }),
        }
    }

    templates
}

pub async fn load_resolve_alcom_templates(
    io: &DefaultEnvironmentIo,
    unity_versions: &[UnityVersion],
) -> Vec<ProjectTemplateInfo> {
    let templates = load_alcom_templates(io).await;

    let mut template_by_id = IndexMap::<String, ProjectTemplateInfo>::new();

    // builtin templates at first
    template_by_id.insert(
        AVATARS_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "VRChat Avatars".into(),
            id: AVATARS_TEMPLATE_ID.into(),
            unity_versions: VRCHAT_UNITY_VERSIONS.into(),
            alcom_template: None,
            source_path: None,
            available: true,
        },
    );
    template_by_id.insert(
        WORLDS_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "VRChat Worlds".into(),
            id: WORLDS_TEMPLATE_ID.into(),
            unity_versions: VRCHAT_UNITY_VERSIONS.into(),
            alcom_template: None,
            source_path: None,
            available: true,
        },
    );
    template_by_id.insert(
        BLANK_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "Blank".into(),
            id: BLANK_TEMPLATE_ID.into(),
            unity_versions: unity_versions.into(),
            alcom_template: None,
            source_path: None,
            available: true,
        },
    );

    // then ALCOM templates
    for (path, value) in templates {
        let id = value.id.clone().unwrap_or_else(|| {
            format!(
                "{}{}",
                UNNAMED_TEMPLATE_PREFIX,
                uuid::Uuid::new_v4().as_simple()
            )
        });
        template_by_id.insert(
            id.clone(),
            ProjectTemplateInfo {
                display_name: value.display_name.clone(),
                id,
                unity_versions: vec![],
                alcom_template: Some(value),
                source_path: Some(path),
                available: false,
            },
        );
    }

    let mut keys_to_update = template_by_id.keys().cloned().collect::<HashSet<_>>();

    // Resolve template dependency
    while {
        let mut updated = false;

        keys_to_update.retain(|k| {
            let template = &template_by_id[k];
            if template.available {
                // the template is already avaiable and valid.
                return false;
            }
            let alcom = template.alcom_template.as_ref().unwrap();
            let Some(base) = template_by_id.get(&alcom.base) else {
                // The template will never become available so remove from keys to update
                return false;
            };

            if !base.available {
                // The base template is not available yet. Retry later
                return true;
            }

            // The base template is available! update this template based on the base template

            let unity_versions = if let Some(unity_filter) = &alcom.unity_version {
                base.unity_versions
                    .iter()
                    .copied()
                    .filter(|x| unity_filter.matches(&x.as_semver()))
                    .collect()
            } else {
                base.unity_versions.clone()
            };

            let template_mut = &mut template_by_id[k];
            template_mut.unity_versions = unity_versions;
            template_mut.available = true;

            updated = true;

            false
        });

        updated
    } {}

    template_by_id.into_values().collect()
}

pub async fn load_alcom_templates(io: &DefaultEnvironmentIo) -> Vec<(PathBuf, AlcomTemplate)> {
    let path = Path::new("vrc-get/templates");
    let mut dir = match io.read_dir(path).await {
        Ok(dir) => dir,
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warn!("failed to read vrc-get/templates directory {path:?}: {e}");
            return Vec::new();
        }
    };
    let mut templates = Vec::new();
    while let Ok(Some(entry)) = dir.try_next().await {
        if entry
            .file_name()
            .as_encoded_bytes()
            .ends_with(b".alcomtemplate")
            && entry
                .file_type()
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
        {
            // The file is alcomtemplate
            let path = path.join(entry.file_name());
            match load_template(io, &path).await {
                Ok(template) => templates.push((path, template)),
                Err(e) => log::warn!(
                    "Error loading template at {path}: {e}",
                    path = path.display()
                ),
            }
        }
    }

    templates
}

pub async fn load_template(io: &DefaultEnvironmentIo, path: &Path) -> io::Result<AlcomTemplate> {
    let mut file = io.open(path).await?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).await?;
    Ok(parse_alcom_template(&buffer)?)
}

async fn copy_recursively(from: PathBuf, to: PathBuf) -> io::Result<u64> {
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = false;
    options.content_only = true;
    match tokio::runtime::Handle::current()
        .spawn_blocking(move || fs_extra::dir::copy(from, to, &options))
        .await
    {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(e)) => match e.kind {
            ErrorKind::Io(io) => Err(io),
            ErrorKind::StripPrefix(_) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            ErrorKind::NotFound => Err(io::Error::new(io::ErrorKind::NotFound, e)),
            ErrorKind::PermissionDenied => Err(io::Error::new(io::ErrorKind::PermissionDenied, e)),
            ErrorKind::AlreadyExists => Err(io::Error::new(io::ErrorKind::AlreadyExists, e)),
            ErrorKind::Interrupted => Err(io::Error::new(io::ErrorKind::Interrupted, e)),
            ErrorKind::InvalidFolder => Err(io::Error::new(io::ErrorKind::NotADirectory, e)),
            ErrorKind::InvalidFile => Err(io::Error::new(io::ErrorKind::IsADirectory, e)),
            ErrorKind::InvalidFileName => Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
            ErrorKind::InvalidPath => Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
            ErrorKind::OsString(_) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            ErrorKind::Other => Err(io::Error::other(e)),
        },
        Err(_) => Err(io::Error::other("background task failed")),
    }
}

#[derive(Debug)]
pub enum CreateProjectErr {
    Io(io::Error),
    NoSuchTemplate,
}

impl std::error::Error for CreateProjectErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreateProjectErr::Io(e) => Some(e),
            CreateProjectErr::NoSuchTemplate => None,
        }
    }
}

impl fmt::Display for CreateProjectErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateProjectErr::Io(e) => fmt::Display::fmt(e, f),
            CreateProjectErr::NoSuchTemplate => f.write_str("no such template or base template"),
        }
    }
}

impl From<io::Error> for CreateProjectErr {
    fn from(err: io::Error) -> Self {
        CreateProjectErr::Io(err)
    }
}

/// Creates a new project based on the specified template
///
/// Caller should have created the empty dir at path.
/// This doesn't resolve dependencies of the project; caller should do.
#[allow(dead_code)]
pub async fn create_project(
    io: &DefaultEnvironmentIo,
    templates: &[ProjectTemplateInfo],
    id: &str,
    project_path: &Path,
    project_name: &str,
    unity_version: UnityVersion,
) -> Result<UnityProject, CreateProjectErr> {
    enum BaseTemplate<'a> {
        BuiltIn(&'static [u8]),
        Blank(UnityVersion),
        Custom(&'a str),
    }

    struct ResolvedTemplateInfo<'a> {
        base_template: BaseTemplate<'a>,
        packages: IndexMap<String, VersionRange>,
        unity_packages: Vec<PathBuf>,
    }

    impl<'a> ResolvedTemplateInfo<'a> {
        fn builtin(tgz: &'static [u8]) -> Self {
            Self {
                base_template: BaseTemplate::BuiltIn(tgz),
                packages: IndexMap::new(),
                unity_packages: Vec::new(),
            }
        }

        fn blank(unity_version: UnityVersion) -> Self {
            Self {
                base_template: BaseTemplate::Blank(unity_version),
                packages: IndexMap::new(),
                unity_packages: Vec::new(),
            }
        }

        fn custom(name: &'a str) -> Self {
            Self {
                base_template: BaseTemplate::Custom(name),
                packages: IndexMap::new(),
                unity_packages: Vec::new(),
            }
        }
    }

    let by_id = templates.iter().map(|t| (t.id.as_str(), t)).collect();

    // resolve template
    let template_info =
        resolve_template(&by_id, id, unity_version).ok_or(CreateProjectErr::NoSuchTemplate)?;

    // extract base template
    info!("Extracting base template");
    match template_info.base_template {
        BaseTemplate::BuiltIn(tgz) => {
            let tar = flate2::read::GzDecoder::new(io::Cursor::new(tgz));
            let mut archive = tar::Archive::new(tar);
            archive.unpack(project_path)?;
        }
        BaseTemplate::Custom(template_name) => {
            let template_path = io.resolve(format!("Templates/{template_name}").as_ref());
            copy_recursively(template_path, project_path.to_path_buf()).await?;
        }
        BaseTemplate::Blank(unity_version) => {
            tokio::fs::create_dir(project_path.join("Assets")).await?;
            tokio::fs::create_dir(project_path.join("ProjectSettings")).await?;
            tokio::fs::create_dir(project_path.join("Packages")).await?;
            tokio::fs::write(
                project_path.join("ProjectSettings/ProjectVersion.txt"),
                format!("m_EditorVersion: {unity_version}\n"),
            )
            .await?;
        }
    }

    // extract unity packages
    for unity_package in template_info.unity_packages {
        info!("extracting unity package: {}", unity_package.display());
        let unity_package = tokio::fs::File::open(unity_package).await?;
        import_unitypackage(project_path, &mut BufReader::new(unity_package.compat())).await?;
    }

    // update ProjectSettings.asset
    info!("Updating ProjectSettings.asset");
    update_project_name_and_guid(project_path, project_name).await?;

    // add dependencies
    info!("Adding dependencies");
    let mut project = UnityProject::load(DefaultProjectIo::new(project_path.into())).await?;

    for (pkg, range) in template_info.packages {
        project.add_dependency_raw(&pkg, DependencyRange::from_version_range(range));
    }

    project.save().await?;

    return Ok(project);

    fn resolve_template<'a>(
        templates: &HashMap<&'a str, &'a ProjectTemplateInfo>,
        id: &'a str,
        unity_version: UnityVersion,
    ) -> Option<ResolvedTemplateInfo<'a>> {
        match id {
            // builtin templates
            AVATARS_TEMPLATE_ID => {
                if unity_version == UNITY_2019_4_31 {
                    Some(ResolvedTemplateInfo::builtin(AVATARS_2019_4_31F1))
                } else if unity_version == UNITY_2022_3_6 {
                    Some(ResolvedTemplateInfo::builtin(AVATARS_2022_3_6F1))
                } else if unity_version == UNITY_2022_3_22 {
                    Some(ResolvedTemplateInfo::builtin(AVATARS_2022_3_22F1))
                } else {
                    panic!("bad version for avatars: {unity_version}")
                }
            }
            WORLDS_TEMPLATE_ID => {
                if unity_version == UNITY_2019_4_31 {
                    Some(ResolvedTemplateInfo::builtin(WORLDS_2019_4_31F1))
                } else if unity_version == UNITY_2022_3_6 {
                    Some(ResolvedTemplateInfo::builtin(WORLDS_2022_3_6F1))
                } else if unity_version == UNITY_2022_3_22 {
                    Some(ResolvedTemplateInfo::builtin(WORLDS_2022_3_22F1))
                } else {
                    panic!("bad version for worlds template: {unity_version}")
                }
            }
            BLANK_TEMPLATE_ID => Some(ResolvedTemplateInfo::blank(unity_version)),
            // vcc templates
            id if id.starts_with(VCC_TEMPLATE_PREFIX) => Some(ResolvedTemplateInfo::custom(
                id.trim_start_matches(VCC_TEMPLATE_PREFIX),
            )),
            // .alcomtemplate files
            id => {
                let template = templates
                    .get(id)?
                    .alcom_template
                    .as_ref()
                    .expect("no .alcomtemplate info");
                let mut resolved = resolve_template(templates, &template.base, unity_version)?;

                for (pkg_id, range) in &template.vpm_dependencies {
                    match resolved.packages.entry(pkg_id.clone()) {
                        Entry::Occupied(mut e) => {
                            let range = range.intersect(e.get());
                            e.insert(range);
                        }
                        Entry::Vacant(e) => {
                            e.insert(range.clone());
                        }
                    }
                }

                (resolved.unity_packages).extend(template.unity_packages.iter().cloned());

                Some(resolved)
            }
        }
    }
}

/// Imports the unitypackage at the `unitypackage`
///
/// This importer holds metadata on the memory and extracts the data to `Library/.temp-dir.<random>/<guid>`,
/// and then move to corresponding directory
pub async fn import_unitypackage(
    project_path: &Path,
    unitypackage: &mut (dyn AsyncBufRead + Unpin + Send + Sync),
) -> io::Result<()> {
    let temp_dir = {
        let library = project_path.join("Library");
        let _ = tokio::fs::create_dir(&library).await;
        loop {
            let temp = library.join(format!(".temp-dir.{}", uuid::Uuid::new_v4().as_simple()));
            match tokio::fs::create_dir(&temp).await {
                Ok(_) => break temp,
                Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
    };

    /// This struct is responsible for removing the project_path synchronously when the
    /// main part panics, or asynchronously when error, or finishes successfully
    struct Scope<'a> {
        temp_dir: &'a Path,
    }

    impl Scope<'_> {
        async fn drop_async(self) -> io::Result<()> {
            let temp_dir = self.temp_dir;
            forget(self);
            tokio::fs::remove_dir_all(temp_dir).await
        }
    }

    impl Drop for Scope<'_> {
        fn drop(&mut self) {
            // ignore error here; we won't double panic
            let _ = std::fs::remove_dir_all(self.temp_dir);
        }
    }

    let scope = Scope {
        temp_dir: &temp_dir,
    };

    let result = import_unitypackage_impl(project_path, &temp_dir, unitypackage).await;
    let remove_result = scope.drop_async().await;

    result.and(remove_result)
}

// the main part of import_unitypackage
// This part does almost all thing of import_unitypackage except for temp_dir management.
async fn import_unitypackage_impl(
    project_path: &Path,
    temp_dir: &Path,
    unitypackage: &mut (dyn AsyncBufRead + Unpin + Send + Sync),
) -> io::Result<()> {
    #[derive(Default)]
    struct UnityPackageEntry {
        // empty means not exists.
        metadata: Vec<u8>,
        // empty means not exists.
        pathname: String,
        has_file: bool,
    }

    let gunzip = GzipDecoder::new(unitypackage);
    let mut untar = TarArchive::new(gunzip);

    type GuidBuf = [u8; 32];
    let mut entries = HashMap::<GuidBuf, UnityPackageEntry>::new();

    while let Some(mut tar_entry) = untar.next_entry().await? {
        let path = tar_entry.header().path_bytes();
        let path = path.as_ref();
        let mut components = Vec::new();
        for component in path.split(|&b| b == b'/' || b == b'\\') {
            match component {
                b"" | b"." => (), // no-op
                b".." => {
                    components.pop();
                }
                c => components.push(c),
            }
        }

        let [guid, filename] = components[..] else {
            continue;
        };
        let Ok(guid) = GuidBuf::try_from(guid) else {
            continue;
        };
        if !guid.iter().all(|x| matches!(x, b'a'..=b'f' | b'0'..=b'9')) {
            // the GUID is not guid
            continue;
        }
        match filename {
            b"asset" => {
                // The contents of the asset.
                let package_entry = entries.entry(guid).or_default();
                if package_entry.has_file {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "duplicate asset for {guid}",
                            guid = std::str::from_utf8(&guid).unwrap()
                        ),
                    ));
                }
                let temp = temp_dir.join(std::str::from_utf8(&guid).unwrap());
                futures::io::copy(
                    tar_entry,
                    &mut tokio::fs::File::create(&temp).await?.compat(),
                )
                .await?;
                package_entry.has_file = true;
            }
            b"asset.meta" => {
                // The metadata of the asset.
                let package_entry = entries.entry(guid).or_default();
                if !package_entry.metadata.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "duplicate asset.meta for {guid}",
                            guid = std::str::from_utf8(&guid).unwrap()
                        ),
                    ));
                }
                tar_entry.read_to_end(&mut package_entry.metadata).await?;
            }
            b"pathname" => {
                // The pathname of the asset.
                let package_entry = entries.entry(guid).or_default();
                if !package_entry.pathname.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "duplicate pathname for {guid}",
                            guid = std::str::from_utf8(&guid).unwrap(),
                        ),
                    ));
                }
                let mut buffer = Vec::new();
                tar_entry.read_to_end(&mut buffer).await?;
                let pathname = String::from_utf8(buffer).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "non-utf8 pathname for {guid}",
                            guid = std::str::from_utf8(&guid).unwrap(),
                        ),
                    )
                })?;
                // Those are filename-banned characters for windows except for portable path separator '/'
                if pathname.contains(['<', '>', ':', '"', '|', '?', '*', '\0']) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "bad pathname for {guid} (banned chars)",
                            guid = std::str::from_utf8(&guid).unwrap(),
                        ),
                    ));
                }
                if pathname.split('/').any(|c| matches!(c, "" | "." | "..")) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "bad pathname for {guid} (possible path traversal)",
                            guid = std::str::from_utf8(&guid).unwrap(),
                        ),
                    ));
                }
                // ignoring paths for non-Assets / Packages
                if !pathname.starts_with("Assets/") && pathname.starts_with("Packages/") {
                    continue;
                }
                package_entry.pathname = pathname;
            }
            _ => continue, // non unitypackage entry
        }
    }

    // validate entire entries
    for (guid, entry) in &entries {
        if entry.pathname.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "no pathname for {guid}",
                    guid = std::str::from_utf8(&guid[..]).unwrap()
                ),
            ));
        }
        if entry.metadata.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "no metadata for {path} ({guid})",
                    path = entry.pathname,
                    guid = std::str::from_utf8(&guid[..]).unwrap()
                ),
            ));
        }
    }

    // actually extract the archive
    for (guid, entry) in &entries {
        let path = project_path.join(&entry.pathname);
        let meta_path = path.with_added_extension("meta");
        let temp_path = temp_dir.join(std::str::from_utf8(&guid[..]).unwrap());
        if entry.has_file {
            tokio::fs::create_dir_all(path.parent().unwrap()).await?;
            try_join!(
                tokio::fs::write(meta_path, &entry.metadata),
                tokio::fs::rename(temp_path, &path),
            )?;
        } else {
            tokio::fs::create_dir_all(path).await?;
            try_join!(tokio::fs::write(meta_path, &entry.metadata))?;
        }
    }

    Ok(())
}

async fn update_project_name_and_guid(path: &Path, project_name: &str) -> io::Result<()> {
    let settings_path = path.join("ProjectSettings/ProjectSettings.asset");
    let mut settings_file = match tokio::fs::File::options()
        .read(true)
        .write(true)
        .open(&settings_path)
        .await
    {
        Ok(file) => file.compat(),
        Err(_) => return Ok(()),
    };

    let mut settings = String::new();
    settings_file.read_to_string(&mut settings).await?;

    fn set_value(buffer: &mut String, finder: &str, value: &str) {
        if let Some(pos) = buffer.find(finder) {
            let before_ws = buffer[..pos]
                .chars()
                .last()
                .map(|x| x.is_ascii_whitespace())
                .unwrap_or(true);
            if before_ws {
                if let Some(eol) = buffer[pos..].find('\n') {
                    let eol = eol + pos;
                    buffer.replace_range((pos + finder.len())..eol, value);
                }
            }
        }
    }

    fn yaml_quote(value: &str) -> String {
        let s = value
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        format!("\"{}\"", s)
    }

    set_value(
        &mut settings,
        "productGUID: ",
        &uuid::Uuid::new_v4().simple().to_string(),
    );
    set_value(&mut settings, "productName: ", &yaml_quote(project_name));

    settings_file.seek(io::SeekFrom::Start(0)).await?;
    settings_file.get_mut().set_len(0).await?;
    settings_file.write_all(settings.as_bytes()).await?;
    settings_file.flush().await?;
    settings_file.get_mut().sync_all().await?;
    drop(settings_file);

    Ok(())
}
