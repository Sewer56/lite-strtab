//! Compilation test for custom wrapper types implementing StringIndex and Offset.

use lite_strtab::{impl_offset, impl_string_index, StringId, StringTableBuilder};

/// StringIndex wrapper type.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ProviderIdx(u16);

/// StringIndex wrapper type.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ModelIdx(u16);

/// Offset wrapper type.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ByteOffset32(u32);

/// Offset wrapper type (usize variant for cross-platform compatibility).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ByteOffsetUsize(usize);

impl_string_index!(ProviderIdx: u16, ModelIdx: u16);
impl_offset!(ByteOffset32: u32, ByteOffsetUsize: usize);

fn _compilation_test() {
    let mut builder: StringTableBuilder<u32, ProviderIdx> =
        StringTableBuilder::new_in(lite_strtab::Global);
    let _id: StringId<ProviderIdx> = builder.try_push("test").unwrap();
    let _table = builder.build();

    let mut builder2: StringTableBuilder<u32, ModelIdx> =
        StringTableBuilder::new_in(lite_strtab::Global);
    let _id2: StringId<ModelIdx> = builder2.try_push("test").unwrap();
    let _table2 = builder2.build();

    let mut builder3: StringTableBuilder<ByteOffset32, ProviderIdx> =
        StringTableBuilder::new_in(lite_strtab::Global);
    let _id3: StringId<ProviderIdx> = builder3.try_push("test").unwrap();
    let _table3 = builder3.build();
}
