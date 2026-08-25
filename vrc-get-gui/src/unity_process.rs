use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Clone, Debug)]
pub(crate) struct UnityProcess {
    pub(crate) project_path: PathBuf,
    pub(crate) process_id: u32,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn find_unity_process_ids_for_project(project_path: &Path) -> Vec<u32> {
    let mut system = System::new();
    refresh_unity_processes(&mut system)
        .into_iter()
        .filter_map(|process| {
            paths_match(&process.project_path, project_path).then_some(process.process_id)
        })
        .collect()
}

pub(crate) fn refresh_unity_processes(system: &mut System) -> Vec<UnityProcess> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::OnlyIfNotSet)
            .without_tasks(),
    );

    system
        .processes()
        .values()
        .filter_map(|process| {
            if !is_unity_process(process) {
                return None;
            }

            let process_project_path = process.cwd()?;
            Some(UnityProcess {
                project_path: process_project_path.to_owned(),
                process_id: process.pid().as_u32(),
            })
        })
        .collect()
}

fn is_unity_process(process: &Process) -> bool {
    process
        .exe()
        .and_then(Path::file_stem)
        .is_some_and(is_unity_name)
}

fn is_unity_name(name: &OsStr) -> bool {
    name.eq_ignore_ascii_case(OsStr::new("Unity"))
}

pub(crate) fn paths_match(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));

    cfg_select! {
        windows => {
            normalize_windows_path(&canonical)
        }
        _ => {
            canonical
        }
    }
}

#[cfg(windows)]
fn normalize_windows_path(path: &Path) -> PathBuf {
    let mut components = path.components();
    let Some(first) = components.next() else {
        return PathBuf::from(path);
    };

    let normalized_first = normalize_first_component(first);
    normalized_first.components().chain(components).collect()
}

#[cfg(windows)]
fn normalize_first_component(component: std::path::Component<'_>) -> PathBuf {
    use std::path::{Component, Prefix};

    let Component::Prefix(prefix) = component else {
        return PathBuf::from(component.as_os_str());
    };

    match prefix.kind() {
        Prefix::VerbatimDisk(disk) | Prefix::Disk(disk) => {
            PathBuf::from(format!("{}:", char::from(disk)))
        }
        Prefix::VerbatimUNC(server, share) | Prefix::UNC(server, share) => {
            let mut path = PathBuf::from(r"\\");
            path.push(server);
            path.push(share);
            path
        }
        _ => PathBuf::from(prefix.as_os_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_unity_executable_name_without_lossy_conversion() {
        assert!(is_unity_name(OsStr::new("Unity")));
        assert!(is_unity_name(OsStr::new("UNITY")));
        assert!(!is_unity_name(OsStr::new("UnityShaderCompiler")));
    }

    #[test]
    fn matches_equivalent_existing_paths_after_canonicalization() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let project_path = temporary_directory.path().join("Avatar");
        let nested_path = project_path.join("Nested");
        std::fs::create_dir_all(&nested_path).unwrap();

        assert!(paths_match(&project_path, &nested_path.join("..")));
    }

    #[test]
    fn follows_file_system_case_sensitivity() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let project_path = temporary_directory.path().join("UnityCaseProbe");
        let differently_cased_path = temporary_directory.path().join("unitycaseprobe");
        std::fs::create_dir(&project_path).unwrap();

        assert_eq!(
            paths_match(&project_path, &differently_cased_path),
            std::fs::canonicalize(&differently_cased_path).is_ok()
        );
    }

    #[test]
    #[cfg(windows)]
    fn matches_verbatim_disk_paths() {
        assert!(paths_match(
            Path::new(r"\\?\C:\Projects\Avatar"),
            Path::new(r"C:\Projects\Avatar")
        ));
    }

    #[test]
    #[cfg(windows)]
    fn matches_verbatim_unc_paths() {
        assert!(paths_match(
            Path::new(r"\\?\UNC\server\share\Avatar"),
            Path::new(r"\\server\share\Avatar")
        ));
    }
}
