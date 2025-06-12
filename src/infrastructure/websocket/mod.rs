//! WebSocket client implementations and data transfer objects.
//!
//! Currently this module provides a client for Binance streaming data.

pub mod binance_client;
pub mod client_handle;
pub mod dto;

// Clean exports - only WebSocket client
pub use binance_client::*;
pub use client_handle::{
    get_global_rest_client, get_global_stream_client, set_global_rest_client,
    set_global_stream_client,
};
pub use dto::*;
