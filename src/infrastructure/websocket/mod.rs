//! WebSocket client implementations and data transfer objects.
//!
//! Currently this module provides a client for Binance streaming data.

pub mod binance_client;
pub mod dto;

// Clean exports - only WebSocket client
pub use binance_client::*;
pub use dto::*;
