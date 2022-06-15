elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use elrond_wasm::{elrond_codec::TopEncode, storage::StorageKey};
use launchpad_common::{
    ongoing_operation::{OngoingOperationType, CONTINUE_OP, STOP_OP},
    random::Random,
    tickets::{TicketRange, WINNING_TICKET},
};

const VEC_MAPPER_START_INDEX: usize = 1;

#[derive(TopEncode, TopDecode)]
pub struct GuaranteedTicketsSelectionOperation<M: ManagedTypeApi + CryptoApi> {
    pub rng: Random<M>,
    pub leftover_tickets: usize,
    pub leftover_ticket_pos_offset: usize,
    pub total_additional_winning_tickets: usize,
}

impl<M: ManagedTypeApi + CryptoApi> Default for GuaranteedTicketsSelectionOperation<M> {
    fn default() -> Self {
        Self {
            rng: Random::default(),
            leftover_tickets: 0,
            leftover_ticket_pos_offset: 1,
            total_additional_winning_tickets: 0,
        }
    }
}

#[elrond_wasm::module]
pub trait GuaranteedTicketWinnersModule:
    launchpad_common::launch_stage::LaunchStageModule
    + launchpad_common::config::ConfigModule
    + launchpad_common::ongoing_operation::OngoingOperationModule
    + launchpad_common::tickets::TicketsModule
{
    #[endpoint(distributeGuaranteedTickets)]
    fn distribute_guaranteed_tickets(&self) -> OperationCompletionStatus {
        self.require_winner_selection_period();

        let flags_mapper = self.flags();
        let mut flags = flags_mapper.get();
        require!(
            flags.were_winners_selected,
            "Must select winners for base launchpad first"
        );
        require!(
            !flags.was_additional_step_completed,
            "Already distributed tickets"
        );

        let mut current_operation: GuaranteedTicketsSelectionOperation<Self::Api> =
            self.load_additional_selection_operation();
        let total_additional_winning_tickets =
            &mut current_operation.total_additional_winning_tickets;
        let leftover_tickets = &mut current_operation.leftover_tickets;
        let max_tier_tickets = self.max_tier_tickets().get();

        let mut users_whitelist = self.max_tier_users();
        let users_list_vec = self.get_users_list_vec_mapper();
        let mut users_left = users_list_vec.len();

        let run_result = if users_left > 0 {
            self.run_while_it_has_gas(|| {
                let current_user = users_list_vec.get(VEC_MAPPER_START_INDEX);
                let _ = users_whitelist.swap_remove(&current_user);
                users_left -= 1;

                let user_confirmed_tickets = self.nr_confirmed_tickets(&current_user).get();
                if user_confirmed_tickets == max_tier_tickets {
                    let ticket_range = self.ticket_range_for_address(&current_user).get();
                    if !self.has_any_winning_tickets(&ticket_range) {
                        self.ticket_status(ticket_range.first_id)
                            .set(WINNING_TICKET);

                        *total_additional_winning_tickets += 1;
                    } else {
                        *leftover_tickets += 1;
                    }
                } else {
                    *leftover_tickets += 1;
                }

                if users_left == 0 {
                    STOP_OP
                } else {
                    CONTINUE_OP
                }
            })
        } else {
            OperationCompletionStatus::Completed
        };

        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_custom_operation(&current_operation);

            return run_result;
        }

        let rng = &mut current_operation.rng;
        let leftover_ticket_pos_offset = &mut current_operation.leftover_ticket_pos_offset;
        let nr_original_winning_tickets = self.nr_winning_tickets().get();
        let last_ticket_pos = self.get_total_tickets();

        let second_op_run_result = self.run_while_it_has_gas(|| {
            if *leftover_tickets == 0 {
                return STOP_OP;
            }

            let current_ticket_pos = nr_original_winning_tickets + *leftover_ticket_pos_offset;
            *leftover_ticket_pos_offset += 1;

            let current_ticket_id = self.ticket_pos_to_id(current_ticket_pos).get();
            let current_ticket_status = self.ticket_status(current_ticket_id).get();
            if current_ticket_status == WINNING_TICKET {
                return CONTINUE_OP;
            }

            let selected_ticket_ok =
                self.try_select_winning_ticket(rng, current_ticket_pos, last_ticket_pos);
            if selected_ticket_ok {
                *leftover_tickets -= 1;
                *total_additional_winning_tickets += 1;
            }

            CONTINUE_OP
        });

        match second_op_run_result {
            OperationCompletionStatus::InterruptedBeforeOutOfGas => {
                self.save_custom_operation(&current_operation);
            }
            OperationCompletionStatus::Completed => {
                flags.was_additional_step_completed = true;
                flags_mapper.set(&flags);

                let ticket_price = self.ticket_price().get();
                let claimable_ticket_payment =
                    ticket_price.amount * (*total_additional_winning_tickets as u32);
                self.claimable_ticket_payment()
                    .update(|claim_amt| *claim_amt += claimable_ticket_payment);
            }
        };

        second_op_run_result
    }

    fn has_any_winning_tickets(&self, ticket_range: &TicketRange) -> bool {
        for ticket_id in ticket_range.first_id..=ticket_range.last_id {
            let ticket_status = self.ticket_status(ticket_id).get();
            if ticket_status == WINNING_TICKET {
                return true;
            }
        }

        false
    }

    fn try_select_winning_ticket(
        &self,
        rng: &mut Random<Self::Api>,
        current_ticket_position: usize,
        last_ticket_position: usize,
    ) -> bool {
        let rand_pos = rng.next_usize_in_range(current_ticket_position, last_ticket_position + 1);

        let winning_ticket_id = self.get_ticket_id_from_pos(rand_pos);
        let current_ticket_id = self.get_ticket_id_from_pos(current_ticket_position);
        if self.is_already_winning_ticket(winning_ticket_id)
            || self.is_already_winning_ticket(current_ticket_id)
        {
            return false;
        }

        self.ticket_pos_to_id(rand_pos).set(current_ticket_id);
        self.ticket_status(winning_ticket_id).set(WINNING_TICKET);

        true
    }

    #[inline]
    fn is_already_winning_ticket(&self, ticket_id: usize) -> bool {
        self.ticket_status(ticket_id).get() == WINNING_TICKET
    }

    fn save_custom_operation(&self, op: &GuaranteedTicketsSelectionOperation<Self::Api>) {
        let mut encoded_data = ManagedBuffer::new();
        let _ = op.top_encode(&mut encoded_data);
        self.save_progress(&OngoingOperationType::AdditionalSelection { encoded_data });
    }

    #[inline]
    fn get_users_list_vec_mapper(&self) -> VecMapper<ManagedAddress> {
        VecMapper::new(StorageKey::new(b"maxTierUsers"))
    }

    #[storage_mapper("maxTierTickets")]
    fn max_tier_tickets(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("maxTierUsers")]
    fn max_tier_users(&self) -> UnorderedSetMapper<ManagedAddress>;
}
