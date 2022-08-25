use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use elrond_wasm::{
    storage::mappers::StorageTokenWrapper,
    types::{Address, BigInt, EsdtLocalRole, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use energy_factory_mock::EnergyFactoryMock;
use energy_query::{Energy, EnergyQueryModule};
use farm::Farm;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};

static REWARD_TOKEN_ID: &[u8] = b"REW-123456";
static FARMING_TOKEN_ID: &[u8] = b"LPTOK-123456";
static FARM_TOKEN_ID: &[u8] = b"FARM-123456";
const DIV_SAFETY: u64 = 1_000_000_000_000;
const PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;
const FARMING_TOKEN_BALANCE: u64 = 100_000_000;

#[test]
fn farm_with_no_boost_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = FarmSetup::new(farm::contract_obj, energy_factory_mock::contract_obj);

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_user = farm_setup.second_user.clone();
    farm_setup.enter_farm(&second_user, second_farm_token_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    farm_setup.b_mock.set_block_nonce(10);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    let first_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(first_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(first_farm_token_amount),
    };
    let first_rewards_amt =
        farm_setup.calculate_rewards(&first_user, first_farm_token_amount, first_attributes);
    let first_expected_rewards_amt = first_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(first_rewards_amt, rust_biguint!(first_expected_rewards_amt));

    let second_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(second_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(second_farm_token_amount),
    };
    let second_rewards_amt =
        farm_setup.calculate_rewards(&second_user, second_farm_token_amount, second_attributes);
    let second_expected_rewards_amt = second_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(
        second_rewards_amt,
        rust_biguint!(second_expected_rewards_amt)
    );
}

pub struct FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub last_farm_token_nonce: u64,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory_mock::ContractObj<DebugApi>, EnergyFactoryBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder> FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub fn new(farm_builder: FarmObjBuilder, energy_factory_builder: EnergyFactoryBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let farm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), farm_builder, "farm.wasm");
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "energy_factory.wasm",
        );

        b_mock
            .execute_tx(&owner, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(REWARD_TOKEN_ID);
                let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);
                let pair_address = managed_address!(&Address::zero());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(&farm_token_id);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
                sc.set_energy_factory_address(managed_address!(
                    energy_factory_wrapper.address_ref()
                ));
            })
            .assert_ok();

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARM_TOKEN_ID,
            &farm_token_roles[..],
        );

        let farming_token_roles = [EsdtLocalRole::Burn];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARMING_TOKEN_ID,
            &farming_token_roles[..],
        );

        let reward_token_roles = [EsdtLocalRole::Mint];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            REWARD_TOKEN_ID,
            &reward_token_roles[..],
        );

        b_mock.set_esdt_balance(
            &first_user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );
        b_mock.set_esdt_balance(
            &second_user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );

        FarmSetup {
            b_mock,
            owner,
            first_user,
            second_user,
            last_farm_token_nonce: 0,
            farm_wrapper,
            energy_factory_wrapper,
        }
    }

    pub fn set_user_energy(
        &mut self,
        user: &Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy)),
                        last_update_epoch,
                        managed_biguint!(locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }

    pub fn enter_farm(&mut self, user: &Address, farming_token_amount: u64) {
        self.last_farm_token_nonce += 1;

        let expected_farm_token_nonce = self.last_farm_token_nonce;
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARMING_TOKEN_ID,
                0,
                &rust_biguint!(farming_token_amount),
                |sc| {
                    let out_farm_token = sc.enter_farm();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                    assert_eq!(
                        out_farm_token.amount,
                        managed_biguint!(farming_token_amount)
                    );
                },
            )
            .assert_ok();
    }

    pub fn calculate_rewards(
        &mut self,
        user: &Address,
        farm_token_amount: u64,
        attributes: FarmTokenAttributes<DebugApi>,
    ) -> num_bigint::BigUint {
        let mut result = rust_biguint!(0);
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let result_managed = sc.calculate_rewards_for_given_position(
                    managed_address!(user),
                    managed_biguint!(farm_token_amount),
                    attributes,
                );
                result =
                    num_bigint::BigUint::from_bytes_be(result_managed.to_bytes_be().as_slice());
            })
            .assert_ok();

        result
    }
}
