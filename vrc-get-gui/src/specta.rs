use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct IndexMapV2<K: std::hash::Hash + Eq, V>(pub IndexMap<K, V>);

#[allow(dead_code)]
trait StringLike: std::hash::Hash + Eq {}

impl StringLike for Box<str> {}
impl StringLike for String {}
