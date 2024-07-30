use crate::state::*;

use stable_deref_trait::StableDeref;
use std::borrow::Cow;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tauri::api::dir::is_dir;
use yoke::{CloneableCart, Yoke, Yokeable};

pub(crate) fn home_dir() -> PathBuf {
    use tauri::api::path::home_dir;
    home_dir().unwrap()
}

pub(crate) fn default_backup_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Backups"]);
    home.to_string_lossy().into_owned()
}

pub(crate) fn project_backup_path<'env>(settings: &'env mut SettingMutRef<'_>) -> &'env str {
    if settings.project_backup_path().is_none() {
        settings.set_project_backup_path(&default_backup_path());
        settings.require_save();
    }

    settings.project_backup_path().unwrap()
}

pub(crate) fn default_default_project_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Projects"]);
    home.to_string_lossy().into_owned()
}

pub(crate) fn default_project_path<'env>(settings: &'env mut SettingMutRef<'_>) -> &'env str {
    if settings.default_project_path().is_none() {
        settings.set_default_project_path(&default_default_project_path());
        settings.require_save();
    }

    settings.default_project_path().unwrap()
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

pub(crate) trait YokeExt<Y: for<'a> Yokeable<'a>, C> {
    fn try_map_project_async<'this, P, F, E, Fut>(
        &'this self,
        f: F,
    ) -> impl Future<Output = Result<Yoke<P, C>, E>>
    where
        P: for<'a> Yokeable<'a>,
        C: CloneableCart + StableDeref,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        <C as Deref>::Target: 'this,
        F: FnOnce(
            &'this <C as Deref>::Target,
            &'this <Y as Yokeable<'this>>::Output,
            PhantomData<&'this ()>,
        ) -> Fut;
}

impl<Y: for<'a> Yokeable<'a>, C> YokeExt<Y, C> for Yoke<Y, C> {
    /// ```rust,compile_fail
    /// # async fn test<Y: for<'a> Yokeable<'a>, C: CloneableCart + StableDeref>(yoke: Yoke<Y, C>) {
    /// let mut outer_arg = None;
    /// yoke.try_map_project_async::<u8, _, (), _>(|_, yokable, _| async move {
    ///     outer_arg = Some(yokable);
    ///     Ok(0)
    /// })
    /// .await;
    /// drop(yoke);
    /// outer_arg.unwrap(); // Errors!
    /// # }
    /// ```
    async fn try_map_project_async<'this, P, F, E, Fut>(&'this self, f: F) -> Result<Yoke<P, C>, E>
    where
        P: for<'a> Yokeable<'a>,
        C: CloneableCart + StableDeref,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        F: FnOnce(
            &'this <C as Deref>::Target,
            &'this <Y as Yokeable<'this>>::Output,
            PhantomData<&'this ()>,
        ) -> Fut,
    {
        let data = f(self.backing_cart(), self.get(), PhantomData).await?;

        unsafe {
            Ok(
                Yoke::new_always_owned(P::make(data))
                    .replace_cart(|()| self.backing_cart().clone()),
            )
        }
    }
}
