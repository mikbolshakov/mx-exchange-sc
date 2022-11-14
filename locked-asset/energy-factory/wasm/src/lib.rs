////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    energy_factory
    (
        callBack
        addLockOptions
        addSCAddressToWhitelist
        getBaseAssetTokenId
        getEnergyAmountForUser
        getEnergyEntryForUser
        getFeesBurnPercentage
        getFeesCollectorAddress
        getFeesFromPenaltyUnlocking
        getLastEpochFeeSentToCollector
        getLegacyLockedTokenId
        getLockOptions
        getLockedTokenId
        getPenaltyAmount
        getTokenUnstakeScAddress
        isPaused
        isSCAddressWhitelisted
        issueLockedToken
        lockTokens
        lockVirtual
        mergeTokens
        migrateOldTokens
        pause
        reduceLockPeriod
        removeLockOptions
        removeSCAddressFromWhitelist
        setFeesBurnPercentage
        setFeesCollectorAddress
        setLockedTokenTransferScAddress
        setTokenUnstakeAddress
        setTransferRoleLockedToken
        setUserEnergyAfterLockedTokenTransfer
        unlockEarly
        unlockTokens
        unpause
        updateEnergyAfterOldTokenUnlock
        updateEnergyForOldTokens
    )
}
