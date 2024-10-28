use serde::de::{
    DeserializeSeed, EnumAccess, Error, IgnoredAny, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt::Formatter;
use std::marker::PhantomData;

pub(crate) struct DedupForwarder<T> {
    inner: T,
    meta: Meta,
}

struct Meta;

impl<T> DedupForwarder<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, meta: Meta }
    }
}

impl Meta {
    fn new_dedup_forwarder<V>(&self, visitor: V) -> DedupForwarder<V> {
        DedupForwarder::new(visitor)
    }
}

impl<'de, T> DeserializeSeed<'de> for DedupForwarder<T>
where
    T: DeserializeSeed<'de>,
{
    type Value = T::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner.deserialize(DedupForwarder::new(deserializer))
    }
}

macro_rules! forward_deserializer {
    ($target:ident: deserialize_unit_struct) => {
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .deserialize_unit_struct(name, self.meta.$target(visitor))
    }
};
($target:ident: deserialize_newtype_struct) => {
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .deserialize_newtype_struct(name, self.meta.$target(visitor))
    }
};
($target:ident: deserialize_tuple_struct) => {
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .deserialize_tuple_struct(name, len, self.meta.$target(visitor))
    }
};
($target:ident: deserialize_struct) => {
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .deserialize_struct(name, fields, self.meta.$target(visitor))
    }
};
($target:ident: deserialize_enum) => {
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .deserialize_enum(name, variants, self.meta.$target(visitor))
    }
    };
    ($target:ident: deserialize_tuple) => {
        fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
            self.inner.deserialize_tuple(len, self.meta.$target(visitor))
        }
    };

    ($target:ident: $ident: ident) => {
        fn $ident<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            self.inner.$ident(self.meta.$target(visitor))
        }
    };

    ($target:ident: $ident: ident $($rest: ident)*) => {
        forward_deserializer!($target: $ident);
        forward_deserializer!($target: $($rest)*);
    };
}

impl<'de, D> Deserializer<'de> for DedupForwarder<D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    forward_deserializer!(
        new_dedup_forwarder:
        deserialize_any
        deserialize_bool
        deserialize_i8
        deserialize_i16
        deserialize_i32
        deserialize_i64
        deserialize_i128
        deserialize_u8
        deserialize_u16
        deserialize_u32
        deserialize_u64
        deserialize_u128
        deserialize_f32
        deserialize_f64
        deserialize_char
        deserialize_str
        deserialize_string
        deserialize_bytes
        deserialize_byte_buf
        deserialize_option
        deserialize_unit
        deserialize_seq
        deserialize_map
        deserialize_identifier
        deserialize_ignored_any
        deserialize_unit_struct
        deserialize_newtype_struct
        deserialize_tuple_struct
        deserialize_struct
        deserialize_enum
        deserialize_tuple
    );

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

macro_rules! visit_type {
    (visit_bool) => { bool };
    (visit_i8) => { i8 };
    (visit_i16) => { i16 };
    (visit_i32) => { i32 };
    (visit_i64) => { i64 };
    (visit_i128) => { i128 };
    (visit_u8) => { u8 };
    (visit_u16) => { u16 };
    (visit_u32) => { u32 };
    (visit_u64) => { u64 };
    (visit_u128) => { u128 };
    (visit_f32) => { f32 };
    (visit_f64) => { f64 };
    (visit_char) => { char };
    (visit_str) => { &str };
    (visit_borrowed_str) => { &'de str };
    (visit_string) => { String };
    (visit_bytes) => { &[u8] };
    (visit_borrowed_bytes) => { &'de [u8] };
    (visit_byte_buf) => { Vec<u8> };
}

macro_rules! forward_visitor {
    ($ident: ident) => {
        fn $ident<E>(self, v: visit_type!($ident)) -> Result<Self::Value, E>
        where
            E: Error,
        {
            self.inner.$ident(v)
        }
    };

    ($ident: ident $($rest: ident)*) => {
        forward_visitor!($ident);
        forward_visitor!($($rest)*);
    };
}

impl<'de, V> Visitor<'de> for DedupForwarder<V>
where
    V: Visitor<'de>,
{
    type Value = V::Value;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        self.inner.expecting(formatter)
    }

    forward_visitor!(
        visit_bool
        visit_i8
        visit_i16
        visit_i32
        visit_i64
        visit_i128
        visit_u8
        visit_u16
        visit_u32
        visit_u64
        visit_u128
        visit_f32
        visit_f64
        visit_char
        visit_str
        visit_borrowed_str
        visit_string
        visit_bytes
        visit_borrowed_bytes
        visit_byte_buf
    );

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.inner.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner.visit_some(DedupForwarder::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.inner.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner
            .visit_newtype_struct(DedupForwarder::new(deserializer))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.inner.visit_seq(DedupForwarder::new(seq))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.inner.visit_map(DedupMapAccess::new(map))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        self.inner.visit_enum(DedupForwarder::new(data))
    }
}

impl<'de, A> SeqAccess<'de> for DedupForwarder<A>
where
    A: SeqAccess<'de>,
{
    type Error = A::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.inner.next_element_seed(DedupForwarder::new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

impl<'de, A> EnumAccess<'de> for DedupForwarder<A>
where
    A: EnumAccess<'de>,
{
    type Error = A::Error;
    type Variant = DedupForwarder<A::Variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (value, variant) = self.inner.variant_seed(DedupForwarder::new(seed))?;
        Ok((value, DedupForwarder::new(variant)))
    }
}

impl<'de, A> VariantAccess<'de> for DedupForwarder<A>
where
    A: VariantAccess<'de>,
{
    type Error = A::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.inner.unit_variant()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.inner.newtype_variant_seed(DedupForwarder::new(seed))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.tuple_variant(len, DedupForwarder::new(visitor))
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner
            .struct_variant(fields, DedupForwarder::new(visitor))
    }
}

struct DedupMapAccess<A> {
    map: A,
    existing_keys: HashSet<String>,
}

impl<'de, A> DedupMapAccess<A>
where
    A: MapAccess<'de>,
{
    fn new(map: A) -> Self {
        Self {
            map,
            existing_keys: HashSet::new(),
        }
    }
}

impl<'de, A> MapAccess<'de> for DedupMapAccess<A>
where
    A: MapAccess<'de>,
{
    type Error = A::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        self.map.next_key_seed(seed)
    }

    fn next_key<K>(&mut self) -> Result<Option<K>, Self::Error>
    where
        K: Deserialize<'de>,
    {
        loop {
            let mut as_str = None;
            break match self
                .map
                .next_key_seed(MapKeySeed::new(PhantomData, &mut as_str))
            {
                Err(e) => Err(e),
                Ok(Some(key)) => {
                    if let Some(as_str) = as_str {
                        if !self.existing_keys.insert(as_str) {
                            let _: IgnoredAny = self.next_value()?;
                            continue;
                        }
                    }
                    Ok(Some(key))
                }
                Ok(None) => Ok(None),
            };
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        self.map.next_value_seed(DedupForwarder::new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.map.size_hint()
    }
}

struct MapKeySeed<'a, T> {
    inner: T,
    as_str: &'a mut Option<String>,
}

impl<'a, T> MapKeySeed<'a, T> {
    fn new(inner: T, as_str: &'a mut Option<String>) -> Self {
        Self { inner, as_str }
    }
}

impl<'de, T> DeserializeSeed<'de> for MapKeySeed<'_, T>
where
    T: DeserializeSeed<'de>,
{
    type Value = T::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner
            .deserialize(MapKeyDeserializer::new(deserializer, self.as_str))
    }
}

struct MapKeyDeserializer<'a, T> {
    inner: T,
    meta: MapKeyMeta<'a>,
}

struct MapKeyMeta<'a> {
    as_str: &'a mut Option<String>,
}

impl<'a, T> MapKeyDeserializer<'a, T> {
    fn new(inner: T, as_str: &'a mut Option<String>) -> Self {
        Self {
            inner,
            meta: MapKeyMeta { as_str },
        }
    }
}

impl<'a> MapKeyMeta<'a> {
    fn new_map_key_deserializer<V>(self, visitor: V) -> MapKeyVisitor<'a, V> {
        MapKeyVisitor::new(visitor, self.as_str)
    }
}

impl<'de, T> Deserializer<'de> for MapKeyDeserializer<'_, T>
where
    T: Deserializer<'de>,
{
    type Error = T::Error;

    forward_deserializer!(
        new_map_key_deserializer:
        deserialize_any
        deserialize_bool
        deserialize_i8
        deserialize_i16
        deserialize_i32
        deserialize_i64
        deserialize_i128
        deserialize_u8
        deserialize_u16
        deserialize_u32
        deserialize_u64
        deserialize_u128
        deserialize_f32
        deserialize_f64
        deserialize_char
        deserialize_str
        deserialize_string
        deserialize_bytes
        deserialize_byte_buf
        deserialize_option
        deserialize_unit
        deserialize_seq
        deserialize_map
        deserialize_identifier
        deserialize_ignored_any
        deserialize_tuple
        deserialize_unit_struct
        deserialize_newtype_struct
        deserialize_tuple_struct
        deserialize_struct
        deserialize_enum
    );

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

struct MapKeyVisitor<'a, T> {
    inner: T,
    as_str: &'a mut Option<String>,
}

impl<'a, T> MapKeyVisitor<'a, T> {
    fn new(inner: T, as_str: &'a mut Option<String>) -> Self {
        Self { inner, as_str }
    }
}

impl<'de, T> Visitor<'de> for MapKeyVisitor<'_, T>
where
    T: Visitor<'de>,
{
    type Value = T::Value;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        self.inner.expecting(formatter)
    }

    forward_visitor!(
        visit_bool
        visit_i8
        visit_i16
        visit_i32
        visit_i64
        visit_i128
        visit_u8
        visit_u16
        visit_u32
        visit_u64
        visit_u128
        visit_f32
        visit_f64
        visit_char
        visit_bytes
        visit_borrowed_bytes
        visit_byte_buf
    );

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        *self.as_str = Some(v.clone());
        self.inner.visit_string(v)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        *self.as_str = Some(v.to_string());
        self.inner.visit_borrowed_str(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        *self.as_str = Some(v.to_string());
        self.inner.visit_str(v)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.inner.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner.visit_some(DedupForwarder::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.inner.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner
            .visit_newtype_struct(DedupForwarder::new(deserializer))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.inner.visit_seq(DedupForwarder::new(seq))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.inner.visit_map(DedupMapAccess::new(map))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        self.inner.visit_enum(DedupForwarder::new(data))
    }
}
