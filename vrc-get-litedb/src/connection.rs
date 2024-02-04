use super::Result;
use crate::connection_string::ConnectionStringFFI;
use crate::error::ErrorFFI;
use crate::lowlevel;
use crate::lowlevel::FFISlice;
use crate::project::{Project, ProjectFFI};

pub use super::connection_string::ConnectionString;

#[derive(Debug)]
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
    ) -> ErrorFFI;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bson::{DateTime, ObjectId};
    use crate::project::ProjectType;
    use std::any::Any;

    const TEST_DB_PATH: &str = "test-resources/vcc.liteDb";

    #[test]
    fn not_found() {
        let path = "test-resources/not-found.liteDb";
        std::fs::remove_file(path).ok();
        let error = ConnectionString::new(path)
            .readonly(true)
            .connect()
            .expect_err("expecting not found");
        assert_eq!(error.kind(), crate::error::ErrorKind::NotFound);
    }

    #[test]
    fn not_found_writable() {
        let path = "test-resources/not-found-writable.liteDb";
        std::fs::remove_file(path).ok();
        ConnectionString::new(path).connect().unwrap();
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_connect() {
        ConnectionString::new(TEST_DB_PATH)
            .readonly(true)
            .connect()
            .unwrap();
    }

    #[test]
    fn test_read() {
        let connection = ConnectionString::new(TEST_DB_PATH)
            .readonly(true)
            .connect()
            .unwrap();

        let projects = connection.get_projects().unwrap();

        assert_eq!(projects.len(), 12);

        // {"_id":{"$oid":"65be38dfcbac18126a694ab2"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 worlds","Type":7,"UnityVersion":"2022.3.6f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:00:15.8020000Z"},"LastModified":{"$date":"2024-02-03T13:00:15.8020000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x38\xdf\xcb\xac\x18\x12\x6a\x69\x4a\xb2"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 worlds",
            ProjectType::WORLDS,
            Some("2022.3.6f1"),
            DateTime::from_millis_since_epoch(1706965215802),
            DateTime::from_millis_since_epoch(1706965215802),
        );

        // {"_id":{"$oid":"65be38f3cbac18126a694ab3"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 avatars","Type":8,"UnityVersion":"2022.3.6f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:00:35.8090000Z"},"LastModified":{"$date":"2024-02-03T13:00:35.8090000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x38\xf3\xcb\xac\x18\x12\x6a\x69\x4a\xb3"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 avatars",
            ProjectType::AVATARS,
            Some("2022.3.6f1"),
            DateTime::from_millis_since_epoch(1706965235809),
            DateTime::from_millis_since_epoch(1706965235809),
        );

        // {"_id":{"$oid":"65be391ecbac18126a694ab4"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 avatars","Type":8,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:01:18.7600000Z"},"LastModified":{"$date":"2024-02-03T13:01:18.7600000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x39\x1e\xcb\xac\x18\x12\x6a\x69\x4a\xb4"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 avatars",
            ProjectType::AVATARS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706965278760),
            DateTime::from_millis_since_epoch(1706965278760),
        );

        // {"_id":{"$oid":"65be394bcbac18126a694ab5"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 worlds","Type":7,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:02:03.1890000Z"},"LastModified":{"$date":"2024-02-03T13:02:03.1890000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x39\x4b\xcb\xac\x18\x12\x6a\x69\x4a\xb5"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 worlds",
            ProjectType::WORLDS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706965323189),
            DateTime::from_millis_since_epoch(1706965323189),
        );

        // {"_id":{"$oid":"65be3d65cbac18126a694ab7"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 unknown","Type":0,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:19:33.5020000Z"},"LastModified":{"$date":"2024-02-03T13:19:33.5020000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x3d\x65\xcb\xac\x18\x12\x6a\x69\x4a\xb7"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 unknown",
            ProjectType::UNKNOWN,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706966373502),
            DateTime::from_millis_since_epoch(1706966373502),
        );

        // {"_id":{"$oid":"65be3f75cbac18126a694ab8"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy avatars","Type":3,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:28:21.9920000Z"},"LastModified":{"$date":"2024-02-03T13:28:21.9920000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x3f\x75\xcb\xac\x18\x12\x6a\x69\x4a\xb8"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy avatars",
            ProjectType::LEGACY_AVATARS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706966901992),
            DateTime::from_millis_since_epoch(1706966901992),
        );

        // {"_id":{"$oid":"65be3fff9854f50fadcd90bc"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy worlds","Type":2,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:30:39.3360000Z"},"LastModified":{"$date":"2024-02-03T13:30:39.3360000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x3f\xff\x98\x54\xf5\x0f\xad\xcd\x90\xbc"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy worlds",
            ProjectType::LEGACY_WORLDS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706967039336),
            DateTime::from_millis_since_epoch(1706967039336),
        );

        // {"_id":{"$oid":"65be40449854f50fadcd90bd"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 vpm starter","Type":9,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:31:48.8900000Z"},"LastModified":{"$date":"2024-02-03T13:31:48.8900000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbe\x40\x44\x98\x54\xf5\x0f\xad\xcd\x90\xbd"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 vpm starter",
            ProjectType::VPM_STARTER,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1706967108890),
            DateTime::from_millis_since_epoch(1706967108890),
        );

        // {"_id":{"$oid":"65bf19d67697f911929636a8"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 sdk2","Type":1,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T05:00:06.3190000Z"},"LastModified":{"$date":"2024-02-04T05:00:06.3190000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbf\x19\xd6\x76\x97\xf9\x11\x92\x96\x36\xa8"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 sdk2",
            ProjectType::LEGACY_SDK2,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1707022806319),
            DateTime::from_millis_since_epoch(1707022806319),
        );

        // {"_id":{"$oid":"65bf2e42cd9c24053deee1be"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm avatars","Type":5,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:14.8080000Z"},"LastModified":{"$date":"2024-02-04T06:27:14.8080000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbf\x2e\x42\xcd\x9c\x24\x05\x3d\xee\xe1\xbe"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm avatars",
            ProjectType::UPM_AVATARS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1707028034808),
            DateTime::from_millis_since_epoch(1707028034808),
        );

        // {"_id":{"$oid":"65bf2e4fcd9c24053deee1bf"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm worlds","Type":4,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:27.3630000Z"},"LastModified":{"$date":"2024-02-04T06:27:27.3630000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbf\x2e\x4f\xcd\x9c\x24\x05\x3d\xee\xe1\xbf"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm worlds",
            ProjectType::UPM_WORLDS,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1707028047363),
            DateTime::from_millis_since_epoch(1707028047363),
        );

        // {"_id":{"$oid":"65bf2e56cd9c24053deee1c0"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm starter","Type":6,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:34.2590000Z"},"LastModified":{"$date":"2024-02-04T06:27:34.2590000Z"}}
        check_exists(
            &projects,
            ObjectId::new(b"\x65\xbf\x2e\x56\xcd\x9c\x24\x05\x3d\xee\xe1\xc0"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm starter",
            ProjectType::UPM_STARTER,
            Some("2019.4.31f1"),
            DateTime::from_millis_since_epoch(1707028054259),
            DateTime::from_millis_since_epoch(1707028054259),
        );

        fn check_exists(
            projects: &[Project],
            id: ObjectId,
            path: &str,
            type_: ProjectType,
            unity_version: Option<&str>,
            created_at: DateTime,
            last_modified: DateTime,
        ) {
            let project = projects
                .iter()
                .filter(|x| x.id() == id)
                .next()
                .expect("not found");

            assert_eq!(project.path(), path);
            assert_eq!(project.unity_version(), unity_version);
            assert_eq!(project.favorite(), false);
            assert_eq!(project.created_at(), created_at);
            assert_eq!(project.last_modified(), last_modified);
            assert_eq!(project.project_type(), type_);
        }
    }
}
