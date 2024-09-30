//! Solana Invoke: harness for invoking functions directly with an eBPF VM.
//!
//! Wraps Solana-rBPF using code from the Agave validator (BPF Loader program,
//! etc.) allowing developers to test single functions without having to
//! re-compile ELFs with a new entrypoint.

mod file;
pub mod serialize;

use {
    crate::serialize::{serialize_params, InvokeSerialize},
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_program_runtime::with_mock_invoke_context,
    solana_rbpf::{
        ebpf::{hash_symbol_name, MM_HEAP_START},
        elf::Executable,
        error::ProgramResult,
        interpreter::Interpreter,
        memory_region::{MemoryMapping, MemoryRegion},
        program::SBPFVersion,
        vm::EbpfVm,
    },
    solana_sdk::feature_set::FeatureSet,
};

/// Invoke a registered function on a compiled eBPF program using the eBPF
/// virtual machine provided by Solana rBPF.
pub fn invoke<T: InvokeSerialize>(
    program_name: &str,
    function_name: &str,
    parameters: T,
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
) -> ProgramResult {
    // Executable setup.
    let elf = file::load_program_elf(program_name);
    let loader = Arc::new(
        create_program_runtime_environment_v1(
            feature_set,
            compute_budget,
            /* reject_deployment_of_broken_elfs */ true,
            /* debugging_features */ false,
        )
        .expect("Failed to create runtime environment."),
    );
    let executable =
        Executable::load(&elf, Arc::clone(&loader)).expect("Failed to create executable.");

    // Obtain the address of the target function.
    let (_, target_pc) = executable
        .get_function_registry()
        .lookup_by_key(hash_symbol_name(function_name.as_bytes()))
        .expect("Failed to find specified function.");

    // Create the memory mapping.
    let sbpf_version = SBPFVersion::V2;
    let config = executable.get_config();
    let (
        _parameter_bytes, // <-- Might be useful.
        regions,
    ) = serialize_params(parameters);
    let stack_size = config.stack_size();
    let heap_size = compute_budget.heap_size;
    let (mut stack, mut heap) = solana_bpf_loader_program::MEMORY_POOL
        .with_borrow_mut(|pool| (pool.get_stack(stack_size), pool.get_heap(heap_size)));
    let memory_mapping = {
        let regions: Vec<MemoryRegion> = vec![
            executable.get_ro_region(),
            MemoryRegion::new_writable_gapped(
                stack
                    .as_slice_mut()
                    .get_mut(..stack_size)
                    .expect("invalid stack size"),
                solana_rbpf::ebpf::MM_STACK_START,
                config.stack_frame_size as u64,
            ),
            MemoryRegion::new_writable(
                heap.as_slice_mut()
                    .get_mut(..heap_size as usize)
                    .expect("invalid heap size"),
                MM_HEAP_START,
            ),
        ]
        .into_iter()
        .chain(regions)
        .collect();
        MemoryMapping::new(regions, config, &sbpf_version)
            .expect("Failed to create memory mapping.")
    };

    // Create the VM.
    with_mock_invoke_context!(invoke_context, transaction_context, vec![]);
    let mut vm = EbpfVm::new(
        loader,
        &sbpf_version,
        &mut invoke_context,
        memory_mapping,
        0,
    );

    // Override the stack pointer to move the program counter to the target
    // program.
    //
    // `ja <target_pc>`:
    //      instruction class       000
    //      operation code          00101
    //      destination register    0000
    //      source register         0000
    //      offset                  <bits of target pc>
    //      immediate               0000000000000000000000000000
    let mut registers = vm.registers;
    registers[1] = solana_rbpf::ebpf::MM_INPUT_START;
    registers[solana_rbpf::ebpf::FRAME_PTR_REG] = vm.stack_pointer;
    registers[11] = {
        let instruction_class: u64 = 0b000;
        let operation_code: u64 = 0b00101;
        let destination_register: u64 = 0b0000;
        let source_register: u64 = 0b0000;
        let offset: u64 = target_pc as u64;
        let immediate: u64 = 0;
        (instruction_class << 61)
            | (operation_code << 56)
            | (destination_register << 52)
            | (source_register << 48)
            | (offset << 16)
            | (immediate)
    };

    // Create the interpreter.
    let mut interpreter = Interpreter::new(&mut vm, &executable, registers);

    // Execute.
    while interpreter.step() {}

    // Return the result.
    vm.program_result
}
