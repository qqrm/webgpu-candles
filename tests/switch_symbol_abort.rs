#![cfg(feature = "render")]
use futures::future::{AbortHandle, Abortable};
use gloo_timers::future::sleep;
use leptos::prelude::*;
use price_chart_wasm::app::{abort_all_streams, current_symbol, stream_abort_handles};
use price_chart_wasm::domain::market_data::Symbol;
use std::time::Duration;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn aborts_old_stream_on_interval_restart() {
    let owner = leptos::prelude::Owner::new();
    owner
        .with(|| {
            leptos::reactive::computed::ScopedFuture::new(async {
                let (handle, reg) = AbortHandle::new_pair();
                current_symbol().set(Symbol::from("BTCUSDT"));
                stream_abort_handles().update(|m| {
                    m.insert(Symbol::from("BTCUSDT"), handle.clone());
                });
                let fut = Abortable::new(sleep(Duration::from_millis(50)), reg);

                abort_all_streams();

                assert!(fut.await.is_err());
                assert!(stream_abort_handles().with(|m| !m.contains_key(&Symbol::from("BTCUSDT"))));
            })
        })
        .await;
}
