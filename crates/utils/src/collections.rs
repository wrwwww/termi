pub type HashMap<K, V> = FxHashMap<K, V>;
pub type HashSet<T> = FxHashSet<T>;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub type IndexSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
pub type TypeIdHashMap<V> = std::collections::HashMap<std::any::TypeId, V, TypeIdHashBuilder>;
pub type TypeIdHashSet = std::collections::HashSet<std::any::TypeId, TypeIdHashBuilder>;

pub use indexmap::Equivalent;
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};
pub use std::collections::*;

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeIdHashBuilder;

impl std::hash::BuildHasher for TypeIdHashBuilder {
    type Hasher = TypeIdHasher;

    fn build_hasher(&self) -> Self::Hasher {
        TypeIdHasher::default()
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeIdHasher {
    value: u64,
}

impl std::hash::Hasher for TypeIdHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // TypeId should only hash its first 8 bytes
        if let Some(bytes) = bytes.get(..8) {
            bytes
                .as_array()
                .map(|&array| self.value = u64::from_ne_bytes(array))
                .unwrap_or_else(|| unreachable!("slice was sliced to 8 bytes"));
        } else {
            panic!(
                "expected a 64-bit value, did you use this hasher with something other than a TypeId?"
            );
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }
}
