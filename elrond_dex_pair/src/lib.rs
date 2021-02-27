#![no_std]

imports!();
derive_imports!();

pub mod liquidity_supply;
pub mod liquidity_pool;

pub use crate::liquidity_supply::*;
pub use crate::liquidity_pool::*;

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[module(LiquiditySupplyModuleImpl)]
    fn supply(&self) -> LiquiditySupplyModuleImpl<T, BigInt, BigUint>;

	#[module(LiquidityPoolModuleImpl)]
    fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, token_a_name: TokenIdentifier, token_b_name: TokenIdentifier, , router_address: Address) {
		self.liquidity_pool().set_token_a_name(&token_a_name);
		self.liquidity_pool().set_token_b_name(&token_b_name);
		self.set_router_address(&router_address);
	}

	#[payable("*")]
    #[endpoint(acceptEsdtPayment)]
    fn accept_payment_endpoint(
        &self,
        #[payment_token] token: TokenIdentifier,
		#[payment] payment: BigUint,
    ) -> SCResult<()> {
        require!(payment > 0, "PAIR: Funds transfer must be a positive number");
        if token != self.liquidity_pool().get_token_a_name() && token != self.liquidity_pool().get_token_b_name() {
            return sc_error!("PAIR: INVALID TOKEN");
        }

        let caller = self.get_caller();
        let mut temporary_funds = self.get_temporary_funds(&caller, &token);
        temporary_funds += payment;
        self.set_temporary_funds(&caller, &token, &temporary_funds);

        Ok(())
    }

	#[endpoint]
	fn update_liquidity_provider_storage(&self,
		user_address: Address,
		actual_token_a: TokenIdentifier,
		actual_token_b: TokenIdentifier,
		amount_a: BigUint,
		amount_b: BigUint) -> SCResult<()> {

		let caller = self.get_caller();

		// require!(caller == self.get_router_address(), "Permission Denied: Only router has access");

		let expected_token_a_name = self.get_token_a_name();
		let expected_token_b_name = self.get_token_b_name();

		require!(actual_token_a == expected_token_a_name, "Wrong token a identifier");
		require!(actual_token_b == expected_token_b_name, "Wrong token b identifier");
		require!(amount_a > 0, "Invalid tokens A amount specified");
		require!(amount_b > 0, "Invalid tokens B amount specified");

	#[endpoint(addLiquidity)]
	fn add_liquidity_endpoint(&self) -> SCResult<()> {

		let caller = self.get_caller();
		let expected_token_a_name = self.liquidity_pool().get_token_a_name();
		let expected_token_b_name = self.liquidity_pool().get_token_b_name();
		let temporary_funds_amount_a = self.get_temporary_funds(&caller, &expected_token_a_name);
		let temporary_funds_amount_b = self.get_temporary_funds(&caller, &expected_token_b_name);

		require!(temporary_funds_amount_a > 0, "PAIR: NO AVAILABLE TOKEN A FUNDS");
		require!(temporary_funds_amount_b > 0, "PAIR: NO AVAILABLE TOKEN B FUNDS");
		
		let result = self.liquidity_pool().add_liquidity(
			temporary_funds_amount_a,
			temporary_funds_amount_b,
		);

		match result {
			SCResult::Ok(()) => {
				let caller = self.get_caller();
				self.clear_temporary_funds(&caller, &expected_token_a_name);
				self.clear_temporary_funds(&caller, &expected_token_b_name);
		Ok(())
			},
			SCResult::Err(err) => {
				// TODO: transfer temporary funds back to caller
				sc_error!(err)
	}
	}
	}


	#[endpoint]
	fn send_tokens_on_swap_success(&self,
		address: Address,
		token_in: TokenIdentifier,
		amount_in: BigUint,
		token_out: TokenIdentifier,
		amount_out: BigUint) -> SCResult<()> {

		let caller = self.get_caller();
		require!(caller == self.get_router_address(), "Permission Denied: Only router has access");
		require!(amount_in > 0, "Invalid tokens amount specified");
		require!(amount_out > 0, "Invalid tokens amount specified");

		let expected_token_a_name = self.get_token_a_name();
		let expected_token_b_name = self.get_token_b_name();
		require!(token_in == expected_token_a_name, "Wrong token a identifier");
		require!(token_out == expected_token_b_name, "Wrong token b identifier");

		let mut reserve = self.get_reserve(&token_in);
		reserve = reserve + amount_in;
		self.set_reserve(&token_in, &reserve);

		reserve = self.get_reserve(&token_out);
		reserve = reserve - amount_out.clone();
		self.set_reserve(&token_out, &reserve);

		//TODO: Check if amount_out is available. If not, send back what was received.
		self.send().direct_esdt(&address, token_out.as_slice(), &amount_out, &[]);
		Ok(())
	}

	#[endpoint]
	fn remove_liquidity(&self, user_address: Address,
		actual_token_a_name: TokenIdentifier,
		actual_token_b_name: TokenIdentifier) -> SCResult<()> {
		let caller = self.get_caller();
		// require!(caller == self.get_router_address(), "Permission Denied: Only router has access");

		require!(
			user_address != Address::zero(),
			"Can't transfer to default address 0x0!"
		);
		let expected_token_a_name = self.get_token_a_name();
		let expected_token_b_name = self.get_token_b_name();

		require!(actual_token_a_name == expected_token_a_name, "Wrong token a identifier");
		require!(actual_token_b_name == expected_token_b_name, "Wrong token b identifier");

		let mut balance_a = self.get_reserve(&expected_token_a_name);
		let mut balance_b = self.get_reserve(&expected_token_b_name);
		let liquidity = self.supply().get_balance_of(&user_address);
		let total_supply = self.supply().get_total_supply();

		let amount_a = (liquidity.clone() * balance_a.clone()) / total_supply.clone();
		let amount_b = (liquidity.clone() * balance_b.clone()) / total_supply;

		require!(&amount_a > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_b > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		
		self.supply()._burn(&user_address, &liquidity);

		self.send().direct_esdt(&user_address, expected_token_a_name.as_slice(), &amount_a, &[]);
		self.send().direct_esdt(&user_address, expected_token_b_name.as_slice(), &amount_b, &[]);

		balance_a -= amount_a;
		balance_b -= amount_b;

		self._update(balance_a, balance_b, expected_token_a_name, expected_token_b_name);

		Ok(())
	}

	#[view]
	fn get_reserves_endpoint(&self) -> SCResult< MultiResult2<BigUint, BigUint> > {
		let caller = self.get_caller();
		require!(caller == self.get_router_address(), "Permission Denied: Only router has access");

		let token_a_name = self.get_token_a_name();
		let token_b_name = self.get_token_b_name();

		let reserve_a = self.get_reserve(&token_a_name);
		let reserve_b = self.get_reserve(&token_b_name);

		Ok( (reserve_a, reserve_b).into() )
	}


	fn _update_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier, amount: BigUint) {
		let mut provider_liquidity = self.get_provider_liquidity(user_address, token_identifier);
		provider_liquidity += amount.clone();
		self.set_provider_liquidity(user_address, token_identifier, &provider_liquidity);

		let mut reserve = self.get_reserve(&token_identifier);
		reserve += amount;
		self.set_reserve(&token_identifier, &reserve);
		

	}


	#[storage_get("router_address")]
	fn get_router_address(&self) -> Address;

	#[storage_set("router_address")]
	fn set_router_address(&self, router_address: &Address);

	#[storage_get("token_a_name")]
	fn get_token_a_name(&self) -> TokenIdentifier;

	#[storage_set("token_a_name")]
	fn set_token_a_name(&self, esdt_token_name: &TokenIdentifier);

	#[storage_get("token_b_name")]
	fn get_token_b_name(&self) -> TokenIdentifier;

	#[storage_set("token_b_name")]
	fn set_token_b_name(&self, esdt_token_name: &TokenIdentifier);

	#[storage_get("reserve")]
	fn get_reserve(&self, esdt_token_name: &TokenIdentifier) -> BigUint;

	#[storage_set("reserve")]
	fn set_reserve(&self, esdt_token_name: &TokenIdentifier, reserve: &BigUint);

	#[view(providerLiquidity)]
	#[storage_get("provider_liquidity")]
	fn get_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("provider_liquidity")]
	fn set_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);
}