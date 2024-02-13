// since this moduke is shared between multiple tests, we need to allow dead code and unused imports
#![allow(dead_code)]
#![allow(unused_imports)]

mod package_collection;
mod virtual_file_system;
mod virtual_project_builder;

pub use package_collection::PackageCollection;
pub use package_collection::PackageCollectionBuilder;
pub use virtual_file_system::VirtualFileSystem;
pub use virtual_project_builder::VirtualProjectBuilder;
