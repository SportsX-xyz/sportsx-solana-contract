use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Platform is currently paused")]
    PlatformPaused,
    
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("Invalid event status")]
    InvalidEventStatus,
    
    #[msg("Event is not active")]
    EventNotActive,
    
    #[msg("Ticket sales not started yet")]
    SalesNotStarted,
    
    #[msg("Ticket sales has ended")]
    SalesEnded,
    
    #[msg("Invalid signature")]
    InvalidSignature,
    
    #[msg("Authorization expired")]
    AuthorizationExpired,
    
    #[msg("Nonce already used")]
    NonceAlreadyUsed,
    
    #[msg("Price mismatch")]
    PriceMismatch,
    
    #[msg("Ticket already checked in")]
    AlreadyCheckedIn,
    
    #[msg("Not ticket owner")]
    NotTicketOwner,
    
    #[msg("Resale limit reached")]
    ResaleLimitReached,
    
    #[msg("Ticket cannot be resold")]
    CannotResellTicket,
    
    #[msg("Listing not active")]
    ListingNotActive,
    
    #[msg("Invalid check-in time")]
    InvalidCheckInTime,
    
    #[msg("Check-in operator not authorized")]
    CheckInOperatorNotAuthorized,
    
    #[msg("Invalid event ID")]
    InvalidEventId,
    
    #[msg("Invalid ticket type ID")]
    InvalidTicketTypeId,
    
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    
    #[msg("Invalid ticket PDA in authorization")]
    InvalidTicketPda,
    
    #[msg("Invalid mint account space")]
    InvalidMintAccountSpace,
}

