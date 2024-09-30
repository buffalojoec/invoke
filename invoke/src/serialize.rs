//! Parameter serialization.

use solana_rbpf::{aligned_memory::AlignedMemory, ebpf::HOST_ALIGN, memory_region::MemoryRegion};

pub trait InvokeSerialize {}

pub(crate) fn serialize_params<T: InvokeSerialize>(
    _params: T,
) -> (AlignedMemory<HOST_ALIGN>, Vec<MemoryRegion>) {
    todo!()
}
