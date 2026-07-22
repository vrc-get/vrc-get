use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::json::{JsonError, JsonValue, try_load_json};

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Clone)]
struct AsJson {
    ignore_official_repository: bool,
    ignore_curated_repository: bool,
}

impl AsJson {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        Ok(Self {
            ignore_official_repository: object
                .get_opt("ignoreOfficialRepository")
                .try_map(JsonValue::into_bool)?
                .unwrap_or(false),
            ignore_curated_repository: object
                .get_opt("ignoreCuratedRepository")
                .try_map(JsonValue::into_bool)?
                .unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VrcGetSettings {
    parsed: AsJson,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let parsed = if let Some(value) =
            try_load_json(io, JSON_PATH.as_ref(), AsJson::from_json_value).await?
        {
            log::warn!("vrc-get specific settings file is experimental feature!");
            value
        } else {
            Default::default()
        };

        Ok(Self { parsed })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.parsed.ignore_official_repository
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.parsed.ignore_curated_repository
    }
}
