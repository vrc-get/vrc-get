use crate::utils::json::{JsonError, JsonValue};

#[derive(Debug, Clone, Default)]
pub(crate) enum YankState {
    #[default]
    NotYanked,
    NoReason,
    Reason(Box<str>),
}

impl YankState {
    pub fn is_yanked(&self) -> bool {
        match self {
            YankState::NotYanked => false,
            YankState::NoReason => true,
            YankState::Reason(_) => true,
        }
    }

    #[allow(dead_code)]
    pub fn reason(&self) -> Option<&str> {
        match self {
            YankState::Reason(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn from_json(json: JsonValue) -> Result<Self, JsonError> {
        if let Ok(bool) = json.clone().into_bool() {
            if bool {
                Ok(YankState::NoReason)
            } else {
                Ok(YankState::NotYanked)
            }
        } else if let Ok(value) = json.clone().into_string() {
            Ok(YankState::Reason(value.into()))
        } else {
            Err(json.unexpected_type_error("Boolean or String"))
        }
    }
}
