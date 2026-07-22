use crate::io;
use crate::io::{AsyncRead, IoTrait};
use futures::AsyncReadExt;
use json_path::JsonPath;
use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, JsonError>;

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Default)]
pub(crate) struct JsonValue {
    value: serde_json::Value,
    path: JsonPath,
}

impl JsonValue {
    pub fn map<T>(self, f: impl FnOnce(JsonValue) -> T) -> Option<T> {
        if let serde_json::Value::Null = self.value {
            None
        } else {
            Some(f(self))
        }
    }

    pub fn try_map<T, E>(
        self,
        f: impl FnOnce(JsonValue) -> std::result::Result<T, E>,
    ) -> std::result::Result<Option<T>, E> {
        self.map(f).transpose()
    }

    pub fn into_object(self) -> Result<JsonObject> {
        match self.value {
            serde_json::Value::Object(value) => Ok(JsonObject {
                value,
                path: self.path,
            }),
            value => Err(unexpected(value, "Object", self.path)),
        }
    }

    pub fn into_string(self) -> Result<String> {
        match self.value {
            serde_json::Value::String(s) => Ok(s),
            value => Err(unexpected(value, "string", self.path)),
        }
    }

    pub fn parse_opt<T, E: Display>(
        self,
        parser: impl FnOnce(String) -> std::result::Result<T, E>,
    ) -> Result<Option<T>> {
        self.try_map(|v| v.parse_req(parser))
    }

    pub fn parse_req<T, E: Display>(
        self,
        parser: impl FnOnce(String) -> std::result::Result<T, E>,
    ) -> Result<T> {
        match self.value {
            serde_json::Value::String(s) => Ok(parser(s).map_err(|x| JsonError::InvalidValue {
                description: x.to_string(),
                at: self.path,
            })?),
            value => Err(unexpected(value, "string", self.path)),
        }
    }

    pub fn into_array(self) -> Result<JsonArray> {
        match self.value {
            serde_json::Value::Array(value) => Ok(JsonArray {
                value,
                path: self.path,
            }),
            value => Err(unexpected(value, "array", self.path)),
        }
    }

    pub fn into_bool(self) -> Result<bool> {
        match self.value {
            serde_json::Value::Bool(value) => Ok(value),
            value => Err(unexpected(value, "bool", self.path)),
        }
    }

    pub fn unexpected_type_error(self, expected: &'static str) -> JsonError {
        unexpected(self.value, expected, self.path)
    }
}

impl Debug for JsonValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl From<JsonObject> for JsonValue {
    fn from(value: JsonObject) -> Self {
        json_value(serde_json::Value::Object(value.value), value.path)
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        json_value(
            serde_json::Value::String(value.to_string()),
            Default::default(),
        )
    }
}

#[derive(Clone, Default)]
pub(crate) struct JsonObject {
    value: Object,
    path: JsonPath,
}

impl JsonObject {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn get_opt(&self, key: &'static str) -> JsonValue {
        match self.value.get(key).cloned() {
            None => json_value(serde_json::Value::Null, self.path.join(key)),
            Some(value) => json_value(value, self.path.join(key)),
        }
    }

    pub fn get_req(&mut self, key: &'static str) -> Result<JsonValue> {
        match self.value.get(key).cloned() {
            None => Err(JsonError::MissingValue {
                key,
                at: self.path.clone(),
            }),
            Some(value) => Ok(json_value(value, self.path.join(key))),
        }
    }

    pub fn get_or_insert_mut(
        &mut self,
        key: &'static str,
        value: JsonValue,
    ) -> &mut serde_json::Value {
        self.value.entry(key).or_insert(value.value)
    }

    pub fn get_mut(&mut self, key: &'static str) -> Option<&mut serde_json::Value> {
        self.value.get_mut(key)
    }

    pub fn insert(&mut self, key: &'static str, value: impl IntoJsonValue) {
        self.value.insert(key.into(), value.into_serde_json());
    }

    pub fn remove(&mut self, key: &'static str) {
        self.value.remove(key);
    }

    pub fn into_keys_parsed<F, T, E>(self, f: F) -> KeysMapped<F>
    where
        F: FnMut(String) -> std::result::Result<T, E>,
        E: Display,
    {
        KeysMapped {
            iter: self.value.into_iter(),
            object_path: self.path,
            f,
        }
    }
}

impl Debug for JsonObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl IntoIterator for JsonObject {
    type Item = (String, JsonValue);
    type IntoIter = JsonObjectOwnedIter;
    fn into_iter(self) -> Self::IntoIter {
        JsonObjectOwnedIter {
            iter: self.value.into_iter(),
            object_path: self.path,
        }
    }
}

impl<K, V> FromIterator<(K, V)> for JsonObject
where
    K: Into<String>,
    V: IntoJsonValue,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            value: iter
                .into_iter()
                .map(|(k, v)| (k.into(), v.into_serde_json()))
                .collect(),
            path: Default::default(),
        }
    }
}

pub(crate) struct JsonObjectOwnedIter {
    iter: serde_json::map::IntoIter,
    object_path: JsonPath,
}

impl Iterator for JsonObjectOwnedIter {
    type Item = (String, JsonValue);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(k, v)| (k.clone(), json_value(v, self.object_path.join(k))))
    }
}

pub(crate) struct KeysMapped<F> {
    f: F,
    iter: serde_json::map::IntoIter,
    object_path: JsonPath,
}

impl<F, T, E> Iterator for KeysMapped<F>
where
    F: FnMut(String) -> std::result::Result<T, E>,
    E: Display,
{
    type Item = Result<(T, JsonValue)>;

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.iter.next()?;
        let path = self.object_path.join(k.clone());
        match (self.f)(k) {
            Err(err) => Some(Err(JsonError::InvalidValue {
                description: err.to_string(),
                at: path,
            })),
            Ok(k) => Some(Ok((k, json_value(v, path)))),
        }
    }
}

pub(crate) struct JsonArray {
    value: Vec<serde_json::Value>,
    path: JsonPath,
}

impl IntoIterator for JsonArray {
    type Item = JsonValue;
    type IntoIter = JsonArrayOwnedIter;
    fn into_iter(self) -> Self::IntoIter {
        JsonArrayOwnedIter {
            iter: self.value.into_iter().enumerate(),
            path: self.path,
        }
    }
}

impl<T: IntoJsonValue> FromIterator<T> for JsonArray {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            value: iter.into_iter().map(|x| x.into_serde_json()).collect(),
            path: Default::default(),
        }
    }
}

pub(crate) struct JsonArrayOwnedIter {
    iter: std::iter::Enumerate<std::vec::IntoIter<serde_json::Value>>,
    path: JsonPath,
}

impl Iterator for JsonArrayOwnedIter {
    type Item = JsonValue;
    fn next(&mut self) -> Option<Self::Item> {
        (self.iter.next()).map(|(i, v)| json_value(v, self.path.join(i)))
    }
}

pub trait IntoJsonValue {
    fn into_serde_json(self) -> serde_json::Value;
}

impl IntoJsonValue for JsonValue {
    fn into_serde_json(self) -> serde_json::Value {
        self.value
    }
}

impl IntoJsonValue for &JsonValue {
    fn into_serde_json(self) -> serde_json::Value {
        self.value.clone()
    }
}

impl IntoJsonValue for JsonObject {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::Object(self.value)
    }
}

impl IntoJsonValue for JsonArray {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::Array(self.value)
    }
}

impl IntoJsonValue for bool {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::Bool(self)
    }
}

impl IntoJsonValue for &str {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self.into())
    }
}

impl IntoJsonValue for &Box<str> {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self.to_string())
    }
}

impl IntoJsonValue for Cow<'_, str> {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self.into_owned())
    }
}

impl IntoJsonValue for String {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self)
    }
}

impl IntoJsonValue for &String {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self.into())
    }
}

impl IntoJsonValue for &PathBuf {
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::String(self.to_string_lossy().into_owned())
    }
}

impl<T> IntoJsonValue for &[T]
where
    for<'a> &'a T: IntoJsonValue,
{
    fn into_serde_json(self) -> serde_json::Value {
        serde_json::Value::Array(self.iter().map(|x| x.into_serde_json()).collect())
    }
}

impl<T> IntoJsonValue for Option<T>
where
    T: IntoJsonValue,
{
    fn into_serde_json(self) -> serde_json::Value {
        self.map_or(serde_json::Value::Null, IntoJsonValue::into_serde_json)
    }
}

mod json_path {
    use std::fmt::{Display, Formatter};
    use std::sync::Arc;

    #[derive(Clone, Default)]
    pub(crate) struct JsonPath {
        inner: Option<Arc<JsonPathInner>>,
    }

    impl JsonPath {
        pub(super) fn join(&self, component: impl Into<JsonPathPart>) -> Self {
            Self {
                inner: Some(Arc::new(JsonPathInner {
                    parent: self.inner.clone(),
                    name: component.into(),
                })),
            }
        }
    }

    pub(super) enum JsonPathPart {
        KeyStatic(&'static str),
        KeyOwned(Box<str>),
        Index(usize),
    }

    impl From<&'static str> for JsonPathPart {
        fn from(value: &'static str) -> Self {
            JsonPathPart::KeyStatic(value)
        }
    }

    impl From<String> for JsonPathPart {
        fn from(value: String) -> Self {
            JsonPathPart::KeyOwned(value.into_boxed_str())
        }
    }

    impl From<usize> for JsonPathPart {
        fn from(value: usize) -> Self {
            JsonPathPart::Index(value)
        }
    }

    struct JsonPathInner {
        name: JsonPathPart,
        parent: Option<Arc<JsonPathInner>>,
    }

    impl Display for JsonPath {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if let Some(inner) = &self.inner {
                inner.fmt(f)
            } else {
                f.write_str("<root>")
            }
        }
    }

    impl Display for JsonPathInner {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if let Some(parent) = &self.parent {
                write!(f, "{parent}.{}", self.name)
            } else {
                self.name.fmt(f)
            }
        }
    }

    impl Display for JsonPathPart {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                JsonPathPart::KeyStatic(key) => f.write_str(key),
                JsonPathPart::KeyOwned(key) => f.write_str(key),
                JsonPathPart::Index(i) => f.write_fmt(format_args!("{}", i)),
            }
        }
    }

    impl std::fmt::Debug for JsonPath {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(&self, f)
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) enum JsonError {
    Io(io::Error),
    InvalidValue {
        description: String,
        at: JsonPath,
    },
    MissingValue {
        key: &'static str,
        at: JsonPath,
    },
    Unexpected {
        value: serde_json::Value,
        expected: &'static str,
        at: JsonPath,
    },
}

fn json_value(value: serde_json::Value, path: JsonPath) -> JsonValue {
    JsonValue { value, path }
}

fn unexpected(value: serde_json::Value, expected: &'static str, at: JsonPath) -> JsonError {
    JsonError::Unexpected {
        value,
        expected,
        at,
    }
}

impl From<io::Error> for JsonError {
    fn from(value: io::Error) -> Self {
        JsonError::Io(value)
    }
}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::Io(io) => write!(f, "io: {io}"),
            JsonError::MissingValue { key, at } => write!(f, "missing value: {key:?}, at: {at}"),
            JsonError::InvalidValue { description, at } => {
                write!(f, "invalid value: {description}, in: {at}")
            }
            JsonError::Unexpected {
                value,
                expected,
                at,
            } => write!(
                f,
                "unexpected value: {value}, expected: {expected}, in: {at}"
            ),
        }
    }
}

impl std::error::Error for JsonError {}

//////////////////////////////////////////////////////////////////////////////////////////

// returns true when no data stored in the file
// typical case is filled with '0' when system crashes, but user may manually reset the file
fn is_blank(buf: &[u8]) -> bool {
    buf.is_empty() || buf.iter().all(|&b| matches!(b, b' ' | 0))
}

async fn read_to_end(mut file: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    let mut vec = Vec::new();
    file.read_to_end(&mut vec).await?;
    Ok(vec)
}

pub(crate) async fn load_json<T>(
    io: &impl IoTrait,
    path: &Path,
    parser: impl FnOnce(JsonValue) -> Result<T>,
) -> io::Result<T> {
    parse_json_file(
        &read_to_end(io.open(path).await?).await?,
        path.display(),
        parser,
    )
}

pub(crate) async fn try_load_json<T>(
    io: &impl IoTrait,
    path: &Path,
    parser: impl FnOnce(JsonValue) -> Result<T>,
) -> io::Result<Option<T>> {
    match io.open(path).await {
        Ok(file) => match read_to_end(file).await? {
            vec if is_blank(&vec) => Ok(None),
            vec => Ok(Some(parse_json_file(&vec, path.display(), parser)?)),
        },
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn parse_json_file<T>(
    slice: &[u8],
    source: impl Display,
    parser: impl FnOnce(JsonValue) -> Result<T>,
) -> io::Result<T> {
    let slice = slice.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(slice);
    let json = serde_json::from_slice(slice).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("syntax error loading {source}: {e}"),
        )
    })?;
    parser(json_value(json, Default::default())).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("syntax error loading {source}: {e}"),
        )
    })
}

pub(crate) fn to_vec_pretty_os_eol(value: &JsonValue) -> io::Result<Vec<u8>> {
    crate::utils::to_vec_pretty_os_eol(&value.value)
}

pub(crate) async fn save_json(
    io: &impl IoTrait,
    path: &Path,
    data: impl Borrow<JsonValue>,
) -> io::Result<()> {
    io.create_dir_all(path.parent().unwrap_or("".as_ref()))
        .await?;
    io.write_atomic(path, &to_vec_pretty_os_eol(data.borrow())?)
        .await?;
    Ok(())
}
