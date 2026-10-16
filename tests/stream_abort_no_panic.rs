#![cfg(feature = "render")]
use futures::future::AbortHandle;
use leptos::prelude::*;
use price_chart_wasm::app::{abort_all_streams, stream_abort_handles};
use price_chart_wasm::domain::market_data::Symbol;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
fn stream_creation_and_abort_is_idempotent() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        let (handle, _) = AbortHandle::new_pair();
        stream_abort_handles().update(|handles| {
            handles.insert(Symbol::from("BTCUSDT"), handle.clone());
        });
        abort_all_streams();
        abort_all_streams();
        assert!(handle.is_aborted());
        assert!(stream_abort_handles().get_untracked().is_empty());
    });
}
