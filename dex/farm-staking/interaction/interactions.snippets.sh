### Deployment Setup Steps ###


# Deploy RIDE Staking Farm with RIDE rewards smart contract
# $ deployStakeFarmContract $STAKING_TOKEN_ID 2500 10

# Register new Farm Tokens
# $ registerFarmToken $STAKING_FARM_ADDRESS StakedToken TOKENSTAKE 18


# Run setup function
# $ StakingSetup

# Top up staking contract with rewards
# $ topUpRewards $STAKING_FARM_ADDRESS $STAKING_TOKEN_ID 0xD3C21BCECCEDA1000000 (1,000,000.000000000000000000 Tokens)

# Start produce rewards
# $ startProduceRewards $STAKING_FARM_ADDRESS

# Enable staking farm contract for interaction
# $ resumeContract $STAKING_FARM_ADDRESS


WALLET_PEM=""
PROXY="https://devnet-gateway.elrond.com"
CHAIN_ID="D"
DIVISION_SAFETY_CONSTANT="0xE8D4A51000" # 10^12 value in HEX


STAKING_TOKEN_ID="" # Fill with staking token identifier
STAKING_FARM_ADDRESS="" # Fill after deploy step with generated address


# params:
#   $1 = Staking Token Identifier (Farming Token)
#   $2 = Max APR
#   $3 = Min unbond epochs
deployStakeFarmContract() {
    staking_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../output/farm-staking.wasm" \
        --arguments $staking_token $DIVISION_SAFETY_CONSTANT $2 $3 \
        --outfile="deploy-stake-farm-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-stake-farm-internal.interaction.json" --expression="data['emitted_tx']['address']")

    echo ""
    echo "Staking Smart Contract address: ${ADDRESS}"
}

# params:
#   $1 = Staking Farm address
#   $2 = Staking Token Identifier (Farming Token)
#   $3 = Max APR
#   $4 = Min unbound epochs
upgradeStakeFarmContract() {
    staking_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract upgrade $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../output/farm-staking.wasm" \
        --arguments $staking_token $DIVISION_SAFETY_CONSTANT $3 $4 \
        --outfile="upgrade-stake-farm-internal.interaction.json" --send || return
}

# params:
#   $1 = Staking Farm address
#   $2 = Staking token name
#   $3 = Staking token ticker
#   $3 = num decimals
registerFarmToken() {
    staking_token_name="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    staking_token_ticker="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=100000000 \
        --value=50000000000000000 \
        --function=registerFarmToken \
        --arguments $staking_token_name $staking_token_ticker $4 \
        --send || return
}

# params:
#   $1 = Staking Farm address
setLocalRolesFarmToken() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=200000000 \
        --function=setLocalRolesFarmToken \
        --send || return
}

# params
#   $1 = Staking Farm address
#   $2 = PerBlockRewards in hex
setPerBlockRewardAmount() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=10000000 \
        --function=setPerBlockRewardAmount \
        --arguments $2 --send || return
}

# params
#   $1 = Staing Farm address
startProduceRewards() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=32499678 \
          --function=startProduceRewards \
          --send || return
}

# params
#   $1 = Staing Farm address
stopProduceRewards() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=32499678 \
          --function=end_produce_rewards \
          --send || return
}

# params
#   $1 = Staing Farm address
#   $2 = Rewards Token Identifier
#   $3 = Rewards Amount in hex
topUpRewards() {
    method_name="0x$(echo -n 'topUpRewards' | xxd -p -u | tr -d '\n')"
    lp_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    
    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=25000000 \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function="ESDTTransfer" \
      --arguments $lp_token $3 $method_name \
      --send || return
}

# params
#   $1 = Staking Farm address
#   $2 = APR value (MAX_PERCENT = 10_000; ex 25% = 2500 / 10_000)
setMaxApr() {
    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=25000000 \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function="setMaxApr" \
      --arguments $2 \
      --send || return
}

# params
#   $1 = Staing Farm address
#   $2 = Address to whitelist
addAddressToWhitelist() {
    whitelist_address="0x$(erdpy wallet bech32 --decode $2)"

    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=25000000 \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function=addAddressToWhitelist \
      --arguments $whitelist_address \
      --send || return
}

# params
#   $1 = Staing Farm address
#   $2 = Min unbond epochs
setMinUnbondEpochs() {
    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=25000000 \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function=setMinUnbondEpochs \
      --arguments $2 \
      --send || return
}

# params:
#   $1 = Staing Farm address
resumeContract() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=50000000 \
        --function=resume --send || return
}

# params
#   $1 = Staing Farm address
pauseContract() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=10000000 \
          --function=pause \
          --send || return
}

# params
#   $1 = Staing Farm address
#   $2 = Gas limit
setTransferExecGasLimit() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=set_transfer_exec_gas_limit \
        --arguments $2 \
        --send || return
}


### VIEW FUNCTIONS ###

getMinUnbondEpochs() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getMinUnbondEpochs || return
}


### Setup ###

StakingSetup() {
    # Set local roles Farm Tokens
    setLocalRolesFarmToken $STAKING_FARM_ADDRESS
    sleep 10

    # Set per block rewards
    setPerBlockRewardAmount $STAKING_FARM_ADDRESS 0x396D211370910000 # (4138000000000000000 = 4.138 Tokens / Block)
    sleep 10

    # end setup with contracts inactive
    pauseContract $STAKING_FARM_ADDRESS
    sleep 10
}