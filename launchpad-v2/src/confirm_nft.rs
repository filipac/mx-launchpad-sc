elrond_wasm::imports!();

use elrond_wasm::storage::StorageKey;

#[elrond_wasm::module]
pub trait ConfirmNftModule:
    launchpad_common::launch_stage::LaunchStageModule
    + launchpad_common::config::ConfigModule
    + launchpad_common::tickets::TicketsModule
    + launchpad_common::permissions::PermissionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::mystery_sft::MysterySftModule
{
    #[payable("*")]
    #[endpoint(confirmNft)]
    fn confirm_nft(&self) {
        self.require_confirmation_period();
        self.require_all_sft_setup_steps_complete();

        let caller = self.blockchain().get_caller();
        let nr_base_launchpad_confirmed = self.nr_confirmed_tickets(&caller).get();
        require!(
            nr_base_launchpad_confirmed > 0,
            "Must confirm launchpad tickets before entering NFT draw"
        );

        let new_user = self.confirmed_nft_user_list().insert(caller);
        require!(new_user, "Already confirmed NFT");

        let payment = self.call_value().payment();
        self.require_exact_nft_cost(&payment);
    }

    #[only_owner]
    #[endpoint(claimNftPayment)]
    fn claim_nft_payment(&self) {
        let mapper = self.claimable_nft_payment();
        let claimable_amount = mapper.get();
        if claimable_amount > 0 {
            let mut payment = self.nft_cost().get();
            payment.amount = claimable_amount;

            let owner = self.blockchain().get_caller();
            self.send().direct(
                &owner,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
                &[],
            );

            mapper.clear();
        }
    }

    fn require_exact_nft_cost(&self, payment: &EsdtTokenPayment<Self::Api>) {
        let nft_cost = self.nft_cost().get();
        require!(
            payment.token_identifier == nft_cost.token_identifier
                && payment.token_nonce == nft_cost.token_nonce
                && payment.amount == nft_cost.amount,
            "Invalid payment"
        );
    }

    fn confirmed_list_to_vec_mapper(&self) -> VecMapper<ManagedAddress> {
        VecMapper::new(StorageKey::new(b"confirmedNftUserList"))
    }

    #[storage_mapper("confirmedNftUserList")]
    fn confirmed_nft_user_list(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("nftCost")]
    fn nft_cost(&self) -> SingleValueMapper<EsdtTokenPayment<Self::Api>>;

    #[storage_mapper("totalAvailableNfts")]
    fn total_available_nfts(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("claimableNftPayment")]
    fn claimable_nft_payment(&self) -> SingleValueMapper<BigUint>;
}
