use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use tauri::api::dir::is_dir;
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::Environment;

pub(crate) fn home_dir() -> PathBuf {
    use tauri::api::path::home_dir;
    home_dir().unwrap()
}

pub(crate) fn default_backup_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Backups"]);
    home.to_string_lossy().into_owned()
}

pub(crate) async fn project_backup_path<'env>(
    env: &'env mut Environment,
    io: &DefaultEnvironmentIo,
) -> io::Result<&'env str> {
    if env.project_backup_path().is_none() {
        env.set_project_backup_path(&default_backup_path());
        env.save(io).await?
    }

    Ok(env.project_backup_path().unwrap())
}

pub(crate) fn default_default_project_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Projects"]);
    home.to_string_lossy().into_owned()
}

pub(crate) async fn default_project_path<'env>(
    env: &'env mut Environment,
    io: &DefaultEnvironmentIo,
) -> io::Result<&'env str> {
    if env.default_project_path().is_none() {
        env.set_default_project_path(&default_default_project_path());
        env.save(io).await?
    }

    Ok(env.default_project_path().unwrap())
}

pub(crate) fn find_existing_parent_dir(path: &Path) -> Option<&Path> {
    let mut parent = path;
    loop {
        if is_dir(parent).unwrap_or(false) {
            return Some(parent);
        }

        match parent.parent() {
            Some(p) => parent = p,
            None => return None,
        }
    }
}

pub(crate) fn find_existing_parent_dir_or_home(path: &Path) -> Cow<Path> {
    find_existing_parent_dir(path)
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(home_dir()))
}
