use crate::infrastructure::websocket::BinanceWebSocketClient;
use futures::lock::Mutex;
use once_cell::sync::OnceCell;
use std::sync::Arc;

static REST_CLIENT: OnceCell<Arc<Mutex<BinanceWebSocketClient>>> = OnceCell::new();
static STREAM_CLIENT: OnceCell<Arc<Mutex<BinanceWebSocketClient>>> = OnceCell::new();

pub fn set_global_rest_client(client: Arc<Mutex<BinanceWebSocketClient>>) {
    let _ = REST_CLIENT.set(client);
}

pub fn set_global_stream_client(client: Arc<Mutex<BinanceWebSocketClient>>) {
    let _ = STREAM_CLIENT.set(client);
}

pub fn get_global_rest_client() -> Option<Arc<Mutex<BinanceWebSocketClient>>> {
    REST_CLIENT.get().cloned()
}

pub fn get_global_stream_client() -> Option<Arc<Mutex<BinanceWebSocketClient>>> {
    STREAM_CLIENT.get().cloned()
}
