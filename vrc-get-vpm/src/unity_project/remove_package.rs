use crate::environment::EmptyEnvironment;
use crate::UnityProject;
use itertools::Itertools;
use std::{fmt, io};

use crate::unity_project::pending_project_changes::RemoveReason;
use crate::unity_project::{pending_project_changes, PendingProjectChanges};

// removing package
impl UnityProject {
    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn remove(&mut self, names: &[&str]) -> Result<(), RemovePackageErr> {
        let changes = self.remove_request(names).await?;

        let values = changes
            .conflicts()
            .values()
            .flat_map(|x| x.conflicting_packages())
            .cloned()
            .collect_vec();

        if !values.is_empty() {
            return Err(RemovePackageErr::ConflictsWith(values));
        }

        self.apply_pending_changes(&EmptyEnvironment, changes)
            .await?;

        Ok(())
    }

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn remove_request(
        &self,
        remove: &[&str],
    ) -> Result<PendingProjectChanges<'static>, RemovePackageErr> {
        use RemovePackageErr::*;

        // check for existence

        let mut not_founds = Vec::new();
        for name in remove.iter().copied() {
            if self.manifest.get_locked(name).is_none() {
                not_founds.push(name.to_owned());
            }
        }

        if !not_founds.is_empty() {
            return Err(NotInstalled(not_founds));
        }

        let mut changes = pending_project_changes::Builder::new();

        // check for conflicts: if some package requires some packages to be removed, it's conflict.

        for dep in self
            .all_packages()
            .filter(|dep| !remove.contains(&dep.name()))
        {
            // TODO: do not conflict if this package is legacy package of installed packages
            for &to_remove in remove {
                if dep.dependencies().contains_key(to_remove) {
                    changes.conflicts(String::from(to_remove), String::from(dep.name()));
                }
            }
        }

        // there's no conflicts. So do remove

        for x in remove {
            changes.remove(x.to_string(), RemoveReason::Requested);
        }

        Ok(changes.build_resolve(self).await)
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
