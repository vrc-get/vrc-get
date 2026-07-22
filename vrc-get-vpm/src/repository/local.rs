use crate::PackageManifest;
use crate::repository::{RemotePackages, RemoteRepository};
use crate::utils::json::{JsonError, JsonObject, JsonValue};
use indexmap::IndexMap;
use url::Url;

#[derive(Debug, Clone)]
pub struct LocalCachedRepository {
    pub(crate) repo: RemoteRepository,
    pub(crate) headers: IndexMap<Box<str>, Box<str>>,
    pub(crate) vrc_get: Option<VrcGetMeta>,
}

impl LocalCachedRepository {
    pub(crate) fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        let repo = RemoteRepository::from_json_value(object.get_opt("repo"))?;
        let headers = (object.get_opt("headers"))
            .try_map(|value| {
                (value.into_object())?
                    .into_iter()
                    .map(|(key, value)| {
                        let value = value.into_string()?;
                        Ok((key.into(), value.into()))
                    })
                    .collect::<Result<_, JsonError>>()
            })?
            .unwrap_or_default();
        let vrc_get = (object.get_opt("vrc-get")).try_map(VrcGetMeta::from_json_value)?;
        Ok(Self {
            repo,
            headers,
            vrc_get,
        })
    }

    pub(crate) fn to_json_value(&self) -> JsonValue {
        let mut object = JsonObject::new();
        object.insert("repo", self.repo.to_json_value());
        if !self.headers.is_empty() {
            object.insert(
                "headers",
                self.headers
                    .iter()
                    .map(|(k, v)| (&**k, v))
                    .collect::<JsonObject>(),
            );
        }
        if let Some(vrc_get) = &self.vrc_get
            && let value = vrc_get.to_json_value()
            && !value.is_empty()
        {
            object.insert("vrc-get", value);
        }
        object.into()
    }

    pub fn new(repo: RemoteRepository, headers: IndexMap<Box<str>, Box<str>>) -> Self {
        Self {
            repo,
            headers,
            vrc_get: None,
        }
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        &self.headers
    }

    pub fn repo(&self) -> &RemoteRepository {
        &self.repo
    }

    pub(crate) fn set_repo(&mut self, mut repo: RemoteRepository) {
        if let Some(id) = self.id() {
            repo.set_id_if_none(|| id.into());
        }
        if let Some(url) = self.url() {
            repo.set_url_if_none(|| url.to_owned());
        }
        self.repo = repo;
    }

    pub(crate) fn set_etag(&mut self, etag: Option<Box<str>>) {
        if let Some(etag) = etag {
            self.vrc_get.get_or_insert_with(Default::default).etag = etag;
        } else if let Some(x) = self.vrc_get.as_mut() {
            x.etag = "".into();
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.repo().url()
    }

    pub fn set_url(&mut self, url: Url) {
        self.repo.set_url(url);
    }

    pub fn id(&self) -> Option<&str> {
        self.repo().id()
    }

    pub fn name(&self) -> Option<&str> {
        self.repo().name()
    }

    pub fn get_versions_of(
        &self,
        package: &str,
    ) -> impl Iterator<Item = &'_ PackageManifest> + use<'_> {
        self.repo().get_versions_of(package)
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ RemotePackages> {
        self.repo().get_packages()
    }
}

#[derive(Debug, Clone, Default)]
pub struct VrcGetMeta {
    pub etag: Box<str>,
}

impl VrcGetMeta {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        Ok(Self {
            etag: (object.get_opt("etag"))
                .try_map(JsonValue::into_string)?
                .unwrap_or_default()
                .into(),
        })
    }

    fn to_json_value(&self) -> JsonObject {
        let mut object = JsonObject::new();
        if !self.etag.is_empty() {
            object.insert("etag", &self.etag);
        }
        object
    }
}
