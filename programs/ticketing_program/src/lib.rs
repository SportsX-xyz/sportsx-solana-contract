use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;
pub mod errors;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod ticketing_program {
    use super::*;

    // ==================== Platform Management ====================
    
    /// Initialize the platform configuration
    pub fn initialize_platform(
        ctx: Context<InitializePlatform>,
        initial_fee_receiver: Pubkey,
        initial_fee_usdc: u64,
        backend_authority: Pubkey,
        event_admin: Pubkey,
    ) -> Result<()> {
        instructions::platform::initialize_platform(
            ctx,
            initial_fee_receiver,
            initial_fee_usdc,
            backend_authority,
            event_admin,
        )
    }

    /// Update platform configuration
    pub fn update_platform_config(
        ctx: Context<UpdatePlatformConfig>,
        new_fee_receiver: Option<Pubkey>,
        new_fee_usdc: Option<u64>,
        new_backend_authority: Option<Pubkey>,
        new_event_admin: Option<Pubkey>,
    ) -> Result<()> {
        instructions::platform::update_platform_config(
            ctx,
            new_fee_receiver,
            new_fee_usdc,
            new_backend_authority,
            new_event_admin,
        )
    }

    /// Toggle platform pause status
    pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
        instructions::platform::toggle_pause(ctx)
    }

    /// Transfer platform authority to a new address (e.g., multisig)
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        instructions::platform::transfer_authority(ctx, new_authority)
    }

    // ==================== Event Management ====================

    /// Create a new event
    pub fn create_event(
        ctx: Context<CreateEvent>,
        event_id: String,
        metadata_uri: String,
        start_time: i64,
        end_time: i64,
        ticket_release_time: i64,
        stop_sale_before: i64,
        resale_fee_rate: u16,
        max_resale_times: u8,
    ) -> Result<()> {
        instructions::event::create_event(
            ctx,
            event_id,
            metadata_uri,
            start_time,
            end_time,
            ticket_release_time,
            stop_sale_before,
            resale_fee_rate,
            max_resale_times,
        )
    }

    /// Update event status
    pub fn update_event_status(
        ctx: Context<UpdateEventStatus>,
        event_id: String,
        new_status: u8,
    ) -> Result<()> {
        instructions::event::update_event_status(ctx, event_id, new_status)
    }

    /// Add a check-in operator for an event
    pub fn add_checkin_operator(
        ctx: Context<AddCheckInOperator>,
        event_id: String,
        operator: Pubkey,
    ) -> Result<()> {
        instructions::event::add_checkin_operator(ctx, event_id, operator)
    }

    /// Remove a check-in operator for an event
    pub fn remove_checkin_operator(
        ctx: Context<RemoveCheckInOperator>,
        event_id: String,
        operator: Pubkey,
    ) -> Result<()> {
        instructions::event::remove_checkin_operator(ctx, event_id, operator)
    }

    // ==================== Ticket Management ====================

    /// Create a new ticket type
    pub fn create_ticket_type(
        ctx: Context<CreateTicketType>,
        event_id: String,
        type_id: String,
        tier_name: String,
        price: u64,
        total_supply: u32,
        color: u32,
    ) -> Result<()> {
        instructions::ticket_management::create_ticket_type(
            ctx,
            event_id,
            type_id,
            tier_name,
            price,
            total_supply,
            color,
        )
    }

    /// Batch mint tickets (lazy minting - increases supply)
    pub fn batch_mint_tickets(
        ctx: Context<BatchMintTickets>,
        event_id: String,
        type_id: String,
        quantity: u32,
    ) -> Result<()> {
        instructions::ticket_management::batch_mint_tickets(ctx, event_id, type_id, quantity)
    }

    // ==================== Purchase Flow ====================

    /// Purchase a ticket with backend authorization
    pub fn purchase_ticket<'info>(
        ctx: Context<'_, '_, '_, 'info, PurchaseTicket<'info>>,
        event_id: String,
        type_id: String,
        authorization_data: purchase::AuthorizationData,
        backend_signature: [u8; 64],
    ) -> Result<()> {
        instructions::purchase::purchase_ticket(
            ctx,
            event_id,
            type_id,
            authorization_data,
            backend_signature,
        )
    }

    // ==================== Marketplace ====================

    /// List a ticket for resale
    pub fn list_ticket(ctx: Context<ListTicket>, resale_price: u64) -> Result<()> {
        instructions::marketplace::list_ticket(ctx, resale_price)
    }

    /// Buy a listed ticket
    pub fn buy_listed_ticket<'info>(
        ctx: Context<'_, '_, '_, 'info, BuyListedTicket<'info>>,
        authorization_data: purchase::AuthorizationData,
        backend_signature: [u8; 64],
    ) -> Result<()> {
        instructions::marketplace::buy_listed_ticket(ctx, authorization_data, backend_signature)
    }

    /// Cancel a ticket listing
    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        instructions::marketplace::cancel_listing(ctx)
    }

    // ==================== Check-in Flow ====================

    /// Check-in a ticket at the event
    pub fn check_in_ticket<'info>(ctx: Context<'_, '_, '_, 'info, CheckInTicket<'info>>, event_id: String) -> Result<()> {
        instructions::checkin::check_in_ticket(ctx, event_id)
    }
}

