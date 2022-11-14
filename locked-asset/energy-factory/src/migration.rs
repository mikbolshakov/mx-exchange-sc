elrond_wasm::imports!();

use crate::energy::Energy;
use common_structs::{Epoch, OldLockedTokenAttributes, UnlockEpochAmountPairs};
use unwrappable::Unwrappable;

const TOKEN_MIGRATION_LOCK_EPOCHS_FACTOR: u64 = 4;

#[elrond_wasm::module]
pub trait SimpleLockMigrationModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + elrond_wasm_modules::pause::PauseModule
    + utils::UtilsModule
{
    #[only_owner]
    #[endpoint(updateEnergyForOldTokens)]
    fn update_energy_for_old_tokens(
        &self,
        user: ManagedAddress,
        total_locked_tokens: BigUint,
        energy_amount: BigUint,
    ) {
        self.require_old_tokens_energy_not_updated(&user);

        self.update_energy(&user, |energy: &mut Energy<Self::Api>| {
            energy.add_energy_raw(total_locked_tokens, energy_amount);
        });

        self.user_updated_old_tokens_energy().add(&user);
    }

    #[endpoint(updateEnergyAfterOldTokenUnlock)]
    fn update_energy_after_old_token_unlock(
        &self,
        original_caller: ManagedAddress,
        epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
    ) {
        self.require_caller_old_factory();
        self.require_old_tokens_energy_was_updated(&original_caller);

        self.update_energy(&original_caller, |energy: &mut Energy<Self::Api>| {
            let current_epoch = self.blockchain().get_block_epoch();
            for pair in epoch_amount_pairs.pairs {
                energy.refund_after_token_unlock(&pair.amount, pair.epoch, current_epoch);
            }
        });
    }

    #[payable("*")]
    #[endpoint(migrateOldTokens)]
    fn migrate_old_tokens(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        self.require_old_tokens_energy_was_updated(&caller);

        let payments = self.get_non_empty_payments();
        let own_sc_address = self.blockchain().get_sc_address();
        let current_epoch = self.blockchain().get_block_epoch();

        let mut output_payments = ManagedVec::new();
        self.update_energy(&caller, |energy| {
            for payment in &payments {
                let new_token =
                    self.migrate_single_old_token(payment, current_epoch, &own_sc_address, energy);
                output_payments.push(new_token);
            }
        });

        self.send().direct_multi(&caller, &output_payments);

        output_payments.into()
    }

    fn migrate_single_old_token(
        &self,
        payment: EsdtTokenPayment,
        current_epoch: Epoch,
        own_sc_address: &ManagedAddress,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let old_token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &payment.token_identifier,
            payment.token_nonce,
        );
        let attributes: OldLockedTokenAttributes<Self::Api> = old_token_data.decode_attributes();
        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        let unlock_epoch_amount_pairs = attributes.get_unlock_amounts_per_epoch(&payment.amount);
        let new_unlock_epoch = self
            .calculate_new_unlock_epoch_for_old_token(&unlock_epoch_amount_pairs, current_epoch);
        for epoch_amount_pair in unlock_epoch_amount_pairs.pairs {
            energy.update_after_unlock_epoch_change(
                &epoch_amount_pair.amount,
                epoch_amount_pair.epoch,
                new_unlock_epoch,
                current_epoch,
            );
        }

        let base_asset = EgldOrEsdtTokenIdentifier::esdt(self.base_asset_token_id().get());
        let original_unlocked_tokens = EgldOrEsdtTokenPayment::new(base_asset, 0, payment.amount);
        let new_locked_tokens = self.lock_tokens(original_unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(new_locked_tokens)
    }

    fn calculate_new_unlock_epoch_for_old_token(
        &self,
        unlock_epoch_amount_pairs: &UnlockEpochAmountPairs<Self::Api>,
        current_epoch: Epoch,
    ) -> Epoch {
        let mut weighted_epochs_sum = BigUint::zero();
        let mut weight_sum = BigUint::zero();
        for epoch_amount_pair in &unlock_epoch_amount_pairs.pairs {
            let lock_epochs_remaining = if epoch_amount_pair.epoch > current_epoch {
                epoch_amount_pair.epoch - current_epoch
            } else {
                0
            };
            weighted_epochs_sum += &epoch_amount_pair.amount * lock_epochs_remaining;
            weight_sum += &epoch_amount_pair.amount;
        }

        let base_lock_epochs_biguint = weighted_epochs_sum / weight_sum;
        let base_lock_epochs = base_lock_epochs_biguint
            .to_u64()
            .unwrap_or_panic::<Self::Api>();

        let new_tentative_unlock_epoch =
            current_epoch + base_lock_epochs * TOKEN_MIGRATION_LOCK_EPOCHS_FACTOR;
        let lock_options = self.get_lock_options();
        let max_lock_option = lock_options.last().unwrap_or_panic::<Self::Api>();
        let new_unlock_epoch =
            core::cmp::min(new_tentative_unlock_epoch, max_lock_option.lock_epochs);

        self.unlock_epoch_to_start_of_month_upper_estimate(new_unlock_epoch)
    }

    fn require_caller_old_factory(&self) {
        let caller = self.blockchain().get_caller();
        let old_factory_address = self.old_locked_asset_factory_address().get();
        require!(
            caller == old_factory_address,
            "May only call this through old factory SC"
        );
    }

    fn require_old_tokens_energy_not_updated(&self, address: &ManagedAddress) {
        require!(
            !self.user_updated_old_tokens_energy().contains(address),
            "Energy for old tokens already updated"
        );
    }

    fn require_old_tokens_energy_was_updated(&self, address: &ManagedAddress) {
        require!(
            self.user_updated_old_tokens_energy().contains(address),
            "Must have energy updated for old tokens first"
        );
    }

    #[storage_mapper("oldLockedAssetFactoryAddress")]
    fn old_locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("userUpdatedOldTokensEnergy")]
    fn user_updated_old_tokens_energy(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
