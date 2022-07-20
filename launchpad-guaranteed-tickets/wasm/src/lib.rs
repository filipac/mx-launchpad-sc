////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    launchpad_guaranteed_tickets
    (
        addTickets
        addUsersToBlacklist
        claimLaunchpadTokens
        claimTicketPayment
        confirmTickets
        depositLaunchpadTokens
        distributeGuaranteedTickets
        filterTickets
        getConfiguration
        getLaunchStageFlags
        getLaunchpadTokenId
        getLaunchpadTokensPerWinningTicket
        getNumberOfConfirmedTicketsForAddress
        getNumberOfWinningTickets
        getNumberOfWinningTicketsForAddress
        getSupportAddress
        getTicketPrice
        getTicketRangeForAddress
        getTotalNumberOfTickets
        getTotalNumberOfTicketsForAddress
        getWinningTicketIdsForAddress
        hasUserClaimedTokens
        isUserBlacklisted
        removeUsersFromBlacklist
        selectWinners
        setClaimStartEpoch
        setConfirmationPeriodStartEpoch
        setLaunchpadTokensPerWinningTicket
        setSupportAddress
        setTicketPrice
        setWinnerSelectionStartEpoch
    )
}

elrond_wasm_node::wasm_empty_callback! {}