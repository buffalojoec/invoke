//! Parameter serialization.

use {
    serde::Serialize,
    solana_rbpf::{
        aligned_memory::AlignedMemory,
        ebpf::{HOST_ALIGN, MM_INPUT_START},
        memory_region::MemoryRegion,
    },
};

pub trait InvokeSerialize {
    fn serialize(&self) -> Vec<u8>;
}

impl<T> InvokeSerialize for T
where
    T: Serialize,
{
    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize.")
    }
}

pub(crate) fn serialize_params<T: InvokeSerialize>(
    params: T,
) -> (AlignedMemory<HOST_ALIGN>, Vec<MemoryRegion>) {
    let data = params.serialize();
    let mut aligned_memory = AlignedMemory::from_slice(&data);
    let regions = vec![MemoryRegion::new_writable(
        aligned_memory.as_slice_mut(),
        MM_INPUT_START,
    )];
    (aligned_memory, regions)
}
