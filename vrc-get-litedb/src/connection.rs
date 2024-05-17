use bson::oid::ObjectId;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::connection_string::ConnectionStringFFI;
use crate::error::ErrorFFI;
use crate::lowlevel;
use crate::lowlevel::{FFISlice, ObjectIdFFI, ToFFI};

pub use super::connection_string::ConnectionString;
use super::Result;

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

    pub fn get_values<T>(&self, collection_name: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        unsafe {
            let mut slice = FFISlice::from_byte_slice(&[]);

            let result = vrc_get_litedb_database_connection_get_all(
                self.ptr.get(),
                FFISlice::from_byte_slice(collection_name.as_ref()),
                &mut slice,
            )
            .into_result();

            let boxed = slice.into_boxed_slice_option();

            result?;

            let vecs = boxed
                .unwrap()
                .into_vec()
                .into_iter()
                .map(|x| x.into_boxed_slice())
                .collect::<Vec<_>>();

            vecs.iter()
                .map(|b| Ok(bson::from_slice(b)?))
                .collect::<Result<Vec<T>>>()
        }
    }

    pub fn update<T>(&self, collection_name: &str, data: &T) -> Result<()>
    where
        T: Serialize,
    {
        unsafe {
            vrc_get_litedb_database_connection_update(
                self.ptr.get(),
                FFISlice::from_byte_slice(collection_name.as_ref()),
                FFISlice::from_byte_slice(&bson::to_vec(data)?),
            )
            .into_result()
        }
    }

    pub fn insert<T>(&self, collection_name: &str, data: &T) -> Result<()>
    where
        T: Serialize,
    {
        unsafe {
            vrc_get_litedb_database_connection_insert(
                self.ptr.get(),
                FFISlice::from_byte_slice(collection_name.as_ref()),
                FFISlice::from_byte_slice(&bson::to_vec(data)?),
            )
            .into_result()
        }
    }

    pub fn delete(&self, collection_name: &str, id: ObjectId) -> Result<()> {
        unsafe {
            vrc_get_litedb_database_connection_delete(
                self.ptr.get(),
                FFISlice::from_byte_slice(collection_name.as_ref()),
                id.to_ffi(),
            )
            .into_result()
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

    fn vrc_get_litedb_database_connection_get_all(
        handle: isize,
        collection_name: FFISlice,
        result: &mut FFISlice<FFISlice>,
    ) -> ErrorFFI;

    fn vrc_get_litedb_database_connection_update(
        handle: isize,
        collection_name: FFISlice,
        data: FFISlice,
    ) -> ErrorFFI;

    fn vrc_get_litedb_database_connection_insert(
        handle: isize,
        collection_name: FFISlice,
        data: FFISlice,
    ) -> ErrorFFI;

    fn vrc_get_litedb_database_connection_delete(
        handle: isize,
        collection_name: FFISlice,
        id: ObjectIdFFI,
    ) -> ErrorFFI;
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) const TEST_DB_PATH: &str = "test-resources/vcc.liteDb";

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
}

#[cfg(test)]
mod project_op_tests {
    use bson::{DateTime, Document};

    use super::tests::*;
    use super::*;

    static COLLECTION: &str = "projects";

    macro_rules! temp_path {
        ($name: literal) => {
            concat!("test-resources/test-project-", $name, ".liteDb")
        };
    }

    #[test]
    fn test_update() {
        let copied = temp_path!("update");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let find = ObjectId::from_bytes(*b"\x65\xbe\x38\xdf\xcb\xac\x18\x12\x6a\x69\x4a\xb2");
        let new_last_modified = DateTime::from_millis(1707061524000);

        let mut project = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == find)
            .unwrap();
        project.insert("LastModified", new_last_modified);

        connection.update(COLLECTION, &project).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();
        let project = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == find)
            .unwrap();
        drop(connection);

        assert_eq!(
            project.get_datetime("LastModified").unwrap(),
            &new_last_modified
        );

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_insert() {
        let copied = temp_path!("insert");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let path = "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\NewProject";
        let now = DateTime::now();
        let new_id = ObjectId::new();
        let new_project = bson::doc! {
            "_id": new_id,
            "Path": path,
            "Type": 7, // WORLDS
            "UnityVersion": "2022.3.6f1",
            "Favorite": false,
            "CreatedAt": now,
            "LastModified": now,
        };

        connection.insert(COLLECTION, &new_project).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();

        let found_project = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == new_id)
            .unwrap();
        drop(connection);

        assert_eq!(found_project.get_str("Path").unwrap(), path);
        assert_eq!(found_project.get_datetime("CreatedAt").unwrap(), &now);
        assert_eq!(found_project.get_datetime("LastModified").unwrap(), &now);

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_delete() {
        let copied = temp_path!("delete");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let project_id = ObjectId::from_bytes(*b"\x65\xbe\x38\xdf\xcb\xac\x18\x12\x6a\x69\x4a\xb2");

        assert!(connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .iter()
            .any(|x| x.get_object_id("_id").unwrap() == project_id));

        connection.delete(COLLECTION, project_id).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();

        assert!(!connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .iter()
            .any(|x| x.get_object_id("_id").unwrap() == project_id));

        drop(connection);

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_read() {
        let connection = ConnectionString::new(TEST_DB_PATH)
            .readonly(true)
            .connect()
            .unwrap();

        let projects = connection.get_values(COLLECTION).unwrap();

        assert_eq!(projects.len(), 12);

        // {"_id":{"$oid":"65be38dfcbac18126a694ab2"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 worlds","Type":7,"UnityVersion":"2022.3.6f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:00:15.8020000Z"},"LastModified":{"$date":"2024-02-03T13:00:15.8020000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x38\xdf\xcb\xac\x18\x12\x6a\x69\x4a\xb2"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 worlds",
            7, // WORLDS
            Some("2022.3.6f1"),
            DateTime::from_millis(1706965215802),
            DateTime::from_millis(1706965215802),
        );

        // {"_id":{"$oid":"65be38f3cbac18126a694ab3"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 avatars","Type":8,"UnityVersion":"2022.3.6f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:00:35.8090000Z"},"LastModified":{"$date":"2024-02-03T13:00:35.8090000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x38\xf3\xcb\xac\x18\x12\x6a\x69\x4a\xb3"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2022 avatars",
            8, // AVATARS
            Some("2022.3.6f1"),
            DateTime::from_millis(1706965235809),
            DateTime::from_millis(1706965235809),
        );

        // {"_id":{"$oid":"65be391ecbac18126a694ab4"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 avatars","Type":8,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:01:18.7600000Z"},"LastModified":{"$date":"2024-02-03T13:01:18.7600000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x39\x1e\xcb\xac\x18\x12\x6a\x69\x4a\xb4"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 avatars",
            8, // AVATARS
            Some("2019.4.31f1"),
            DateTime::from_millis(1706965278760),
            DateTime::from_millis(1706965278760),
        );

        // {"_id":{"$oid":"65be394bcbac18126a694ab5"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 worlds","Type":7,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:02:03.1890000Z"},"LastModified":{"$date":"2024-02-03T13:02:03.1890000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x39\x4b\xcb\xac\x18\x12\x6a\x69\x4a\xb5"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 worlds",
            7, // WORLDS
            Some("2019.4.31f1"),
            DateTime::from_millis(1706965323189),
            DateTime::from_millis(1706965323189),
        );

        // {"_id":{"$oid":"65be3d65cbac18126a694ab7"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 unknown","Type":0,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:19:33.5020000Z"},"LastModified":{"$date":"2024-02-03T13:19:33.5020000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x3d\x65\xcb\xac\x18\x12\x6a\x69\x4a\xb7"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 unknown",
            0, // UNKNOWN
            Some("2019.4.31f1"),
            DateTime::from_millis(1706966373502),
            DateTime::from_millis(1706966373502),
        );

        // {"_id":{"$oid":"65be3f75cbac18126a694ab8"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy avatars","Type":3,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:28:21.9920000Z"},"LastModified":{"$date":"2024-02-03T13:28:21.9920000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x3f\x75\xcb\xac\x18\x12\x6a\x69\x4a\xb8"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy avatars",
            3, // LEGACY_AVATARS
            Some("2019.4.31f1"),
            DateTime::from_millis(1706966901992),
            DateTime::from_millis(1706966901992),
        );

        // {"_id":{"$oid":"65be3fff9854f50fadcd90bc"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy worlds","Type":2,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:30:39.3360000Z"},"LastModified":{"$date":"2024-02-03T13:30:39.3360000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x3f\xff\x98\x54\xf5\x0f\xad\xcd\x90\xbc"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 legacy worlds",
            2, // LEGACY_WORLDS
            Some("2019.4.31f1"),
            DateTime::from_millis(1706967039336),
            DateTime::from_millis(1706967039336),
        );

        // {"_id":{"$oid":"65be40449854f50fadcd90bd"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 vpm starter","Type":9,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-03T13:31:48.8900000Z"},"LastModified":{"$date":"2024-02-03T13:31:48.8900000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbe\x40\x44\x98\x54\xf5\x0f\xad\xcd\x90\xbd"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 vpm starter",
            9, // VPM_STARTER
            Some("2019.4.31f1"),
            DateTime::from_millis(1706967108890),
            DateTime::from_millis(1706967108890),
        );

        // {"_id":{"$oid":"65bf19d67697f911929636a8"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 sdk2","Type":1,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T05:00:06.3190000Z"},"LastModified":{"$date":"2024-02-04T05:00:06.3190000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbf\x19\xd6\x76\x97\xf9\x11\x92\x96\x36\xa8"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 sdk2",
            1, // LEGACY_SDK2
            Some("2019.4.31f1"),
            DateTime::from_millis(1707022806319),
            DateTime::from_millis(1707022806319),
        );

        // {"_id":{"$oid":"65bf2e42cd9c24053deee1be"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm avatars","Type":5,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:14.8080000Z"},"LastModified":{"$date":"2024-02-04T06:27:14.8080000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbf\x2e\x42\xcd\x9c\x24\x05\x3d\xee\xe1\xbe"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm avatars",
            5, // UPM_AVATARS
            Some("2019.4.31f1"),
            DateTime::from_millis(1707028034808),
            DateTime::from_millis(1707028034808),
        );

        // {"_id":{"$oid":"65bf2e4fcd9c24053deee1bf"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm worlds","Type":4,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:27.3630000Z"},"LastModified":{"$date":"2024-02-04T06:27:27.3630000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbf\x2e\x4f\xcd\x9c\x24\x05\x3d\xee\xe1\xbf"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm worlds",
            4, // UPM_WORLDS
            Some("2019.4.31f1"),
            DateTime::from_millis(1707028047363),
            DateTime::from_millis(1707028047363),
        );

        // {"_id":{"$oid":"65bf2e56cd9c24053deee1c0"},"Path":"C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm starter","Type":6,"UnityVersion":"2019.4.31f1","Favorite":false,"CreatedAt":{"$date":"2024-02-04T06:27:34.2590000Z"},"LastModified":{"$date":"2024-02-04T06:27:34.2590000Z"}}
        check_exists(
            &projects,
            ObjectId::from_bytes(*b"\x65\xbf\x2e\x56\xcd\x9c\x24\x05\x3d\xee\xe1\xc0"),
            "C:\\Users\\anata\\AppData\\Local\\VRChatProjects\\VCC Config Test 2019 upm starter",
            6, // UPM_STARTER
            Some("2019.4.31f1"),
            DateTime::from_millis(1707028054259),
            DateTime::from_millis(1707028054259),
        );

        fn check_exists(
            projects: &[Document],
            id: ObjectId,
            path: &str,
            type_: i32,
            unity_version: Option<&str>,
            created_at: DateTime,
            last_modified: DateTime,
        ) {
            let project = projects
                .iter()
                .find(|x| x.get_object_id("_id").unwrap() == id)
                .expect("not found");

            assert_eq!(project.get_str("Path").unwrap(), path);
            assert_eq!(project.get_i32("Type").unwrap(), type_);
            assert_eq!(project.get_str("UnityVersion").ok(), unity_version);
            assert!(!project.get_bool("Favorite").unwrap());
            assert_eq!(project.get_datetime("CreatedAt").unwrap(), &created_at);
            assert_eq!(
                project.get_datetime("LastModified").unwrap(),
                &last_modified
            );
        }
    }
}

#[cfg(test)]
mod unity_versions_op_tests {
    use super::tests::*;
    use super::*;
    use bson::{doc, Document};

    static COLLECTION: &str = "unityVersions";

    macro_rules! temp_path {
        ($name: literal) => {
            concat!("test-resources/test-unity-version-", $name, ".liteDb")
        };
    }

    #[test]
    fn test_update() {
        let copied = temp_path!("update");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let find = ObjectId::from_bytes(*b"\x65\xbe\x38\xa0\xcb\xac\x18\x12\x6a\x69\x4a\xb1");

        let mut version = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == find)
            .unwrap();

        assert!(version.get_bool("LoadedFromHub").unwrap());
        version.insert("LoadedFromHub", false);

        connection.update(COLLECTION, &version).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();
        let version = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == find)
            .unwrap();
        drop(connection);

        assert!(!version.get_bool("LoadedFromHub").unwrap());

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_insert() {
        let copied = temp_path!("insert");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let new_id = ObjectId::new();
        let new_path = "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.19f1\\Editor\\Unity.exe";
        let new_version = "2022.3.6f1";
        let new_loaded_from_hub = false;
        let new_document = doc! {
            "_id": new_id,
            "Path": new_path,
            "Version": new_version,
            "LoadedFromHub": new_loaded_from_hub,
        };

        connection.insert(COLLECTION, &new_document).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();

        let found_project = connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .find(|x| x.get_object_id("_id").unwrap() == new_id)
            .unwrap();
        drop(connection);

        assert_eq!(found_project.get_str("Path").unwrap(), new_path);
        assert_eq!(found_project.get_str("Version").unwrap(), new_version);
        assert_eq!(
            found_project.get_bool("LoadedFromHub").unwrap(),
            new_loaded_from_hub
        );

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_delete() {
        let copied = temp_path!("delete");
        std::fs::remove_file(copied).ok();
        std::fs::copy(TEST_DB_PATH, copied).unwrap();
        let connection = ConnectionString::new(copied).connect().unwrap();
        let project_id = ObjectId::from_bytes(*b"\x65\xbe\x38\xa0\xcb\xac\x18\x12\x6a\x69\x4a\xb1");

        assert!(connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .any(|x| x.get_object_id("_id").unwrap() == project_id));

        connection.delete(COLLECTION, project_id).unwrap();

        drop(connection);

        let connection = ConnectionString::new(copied)
            .readonly(true)
            .connect()
            .unwrap();

        assert!(!connection
            .get_values::<Document>(COLLECTION)
            .unwrap()
            .into_iter()
            .any(|x| x.get_object_id("_id").unwrap() == project_id));

        drop(connection);

        // teardown
        std::fs::remove_file(copied).ok();
    }

    #[test]
    fn test_read() {
        let connection = ConnectionString::new(TEST_DB_PATH)
            .readonly(true)
            .connect()
            .unwrap();

        let versions = connection.get_values(COLLECTION).unwrap();

        assert_eq!(versions.len(), 2);

        // {"_id": {"$oid": "65be38a0cbac18126a694ab1"},"Path": "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.6f1\\Editor\\Unity.exe","Version": "2022.3.6f1","LoadedFromHub": true}
        check_exists(
            &versions,
            ObjectId::from_bytes(*b"\x65\xbe\x38\xa0\xcb\xac\x18\x12\x6a\x69\x4a\xb1"),
            "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.6f1\\Editor\\Unity.exe",
            "2022.3.6f1",
        );

        // {"_id": {"$oid": "65be3f989854f50fadcd90bb"},"Path": "C:\\Program Files\\Unity\\Hub\\Editor\\2019.4.31f1\\Editor\\Unity.exe","Version": "2019.4.31f1","LoadedFromHub": true}
        check_exists(
            &versions,
            ObjectId::from_bytes(*b"\x65\xbe\x3f\x98\x98\x54\xf5\x0f\xad\xcd\x90\xbb"),
            "C:\\Program Files\\Unity\\Hub\\Editor\\2019.4.31f1\\Editor\\Unity.exe",
            "2019.4.31f1",
        );

        fn check_exists(versions: &[Document], id: ObjectId, path: &str, version: &str) {
            let project = versions
                .iter()
                .find(|x| x.get_object_id("_id").unwrap() == id)
                .expect("not found");

            assert_eq!(project.get_str("Path").unwrap(), path);
            assert_eq!(project.get_str("Version").unwrap(), version);
            assert!(project.get_bool("LoadedFromHub").unwrap());
        }
    }
}
