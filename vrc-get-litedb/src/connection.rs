use super::Result;
use crate::connection_string::{ConnectionString, ConnectionStringFFI};
use crate::error::LiteDbErrorFFI;
use crate::lowlevel;
use crate::lowlevel::FFISlice;
use crate::project::{Project, ProjectFFI};

pub struct DatabaseConnection {
    ptr: lowlevel::GcHandle,
}

impl DatabaseConnection {
    pub(crate) fn connect(string: &ConnectionString) -> Result<DatabaseConnection> {
        unsafe {
            vrc_get_litedb_database_connection_new(&ConnectionStringFFI::from(string))
                .into_result()
                .map(|ptr| DatabaseConnection { ptr })
        }
    }

    pub fn get_projects(&self) -> Result<Box<[Project]>> {
        unsafe {
            let mut slice = FFISlice::<ProjectFFI>::from_byte_slice(&[]);

            let result =
                vrc_get_litedb_database_connection_get_projects(self.ptr.get(), &mut slice)
                    .into_result();
            let boxed = slice.as_boxed_byte_slice_option();

            result?; // return if error

            return Ok(boxed
                .unwrap()
                .into_vec()
                .into_iter()
                .map(|x| Project::from_ffi(x))
                .collect());
        }
    }
}

impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        unsafe {
            vrc_get_litedb_database_connection_dispose(self.ptr.get());
        }
    }
}

// C# functions
extern "C" {
    fn vrc_get_litedb_database_connection_new(
        string: &ConnectionStringFFI,
    ) -> super::error::HandleErrorResult;
    fn vrc_get_litedb_database_connection_dispose(ptr: isize);
    fn vrc_get_litedb_database_connection_get_projects(
        ptr: isize,
        out: &mut FFISlice<ProjectFFI>,
    ) -> LiteDbErrorFFI;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect() {
        ConnectionString::new("vcc.litedb")
            .readonly(true)
            .connect()
            .unwrap();
    }

    #[test]
    fn test_read() {
        let connection = ConnectionString::new("vcc.litedb")
            .readonly(true)
            .connect()
            .unwrap();

        let projects = connection.get_projects().unwrap();

        dbg!(projects);
    }
}
