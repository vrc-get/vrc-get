use crate::utils::*;
use crate::UnityProject;
use futures::future::try_join_all;
use futures::prelude::*;
use std::{fmt, io};
use tokio::fs::remove_dir_all;

// removing package
impl UnityProject {
    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn remove(&mut self, names: &[&str]) -> Result<(), RemovePackageErr> {
        use RemovePackageErr::*;

        // check for existence

        let mut not_founds = Vec::new();
        for name in names.iter().copied() {
            if self.manifest.get_locked(name).is_none() {
                not_founds.push(name.to_owned());
            }
        }

        if !not_founds.is_empty() {
            return Err(NotInstalled(not_founds));
        }

        // check for conflicts: if some package requires some packages to be removed, it's conflict.

        let conflicts = self
            .all_dependencies()
            .filter(|dep| !names.contains(&dep.name()))
            .filter(|dep| names.iter().any(|x| dep.dependencies().contains_key(*x)))
            .map(|dep| String::from(dep.name()))
            .collect::<Vec<_>>();

        if !conflicts.is_empty() {
            return Err(ConflictsWith(conflicts));
        }

        // there's no conflicts. So do remove

        self.manifest.remove_packages(names.iter().copied());
        try_join_all(names.iter().map(|name| {
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum RemovePackageErr {
    Io(io::Error),
    NotInstalled(Vec<String>),
    ConflictsWith(Vec<String>),
}

impl fmt::Display for RemovePackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RemovePackageErr::*;
        match self {
            Io(ioerr) => fmt::Display::fmt(ioerr, f),
            NotInstalled(names) => {
                f.write_str("the following packages are not installed: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                for name in iter {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
            ConflictsWith(names) => {
                f.write_str("removing packages conflicts with the following packages: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                for name in iter {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RemovePackageErr {}

impl From<io::Error> for RemovePackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
