use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

// Mocked-out SBF entrypoint.
solana_program::entrypoint!(process);
fn process(_program_id: &Pubkey, _accounts: &[AccountInfo], _input: &[u8]) -> ProgramResult {
    // No-Op.
    Ok(())
}

// Test function. Just sets the return code of the VM to 1.
#[no_mangle]
pub extern "C" fn my_registered_function(_input: *mut u8) {
    unsafe {
        core::arch::asm!("lddw r0, 1");
    }
}

#[cfg(test)]
mod tests {
    use {
        solana_compute_budget::compute_budget::ComputeBudget, solana_sdk::feature_set::FeatureSet,
    };

    #[test]
    fn test() {
        solana_invoke::invoke(
            "test_program",
            "my_registered_function",
            &[] as &[u8],
            &FeatureSet::all_enabled(),
            &ComputeBudget::default(),
        );
    }
}
