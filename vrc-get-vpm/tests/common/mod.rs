// since this moduke is shared between multiple tests, we need to allow dead code and unused imports
#![allow(dead_code)]
#![allow(unused_imports)]

mod package_collection;
mod virtual_file_system;
mod virtual_project_builder;

pub(crate) use package_collection::PackageCollection;
pub(crate) use package_collection::PackageCollectionBuilder;
pub(crate) use virtual_file_system::VirtualFileSystem;
pub(crate) use virtual_project_builder::VirtualProjectBuilder;
