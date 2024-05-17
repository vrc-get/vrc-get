use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::{DataType, DefOpts, ExportError, Type};

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
#[serde(transparent)]
pub struct IndexMapV2<K: std::hash::Hash + Eq, V>(pub IndexMap<K, V>);

trait StringLike: std::hash::Hash + Eq {}

impl StringLike for Box<str> {}
impl StringLike for String {}

impl<K: StringLike, V: StringLike> Type for IndexMapV2<K, V> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Record(Box::new((
            String::inline(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
            String::inline(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
        ))))
    }

    fn reference(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Record(Box::new((
            String::reference(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
            String::reference(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
        ))))
    }
}
