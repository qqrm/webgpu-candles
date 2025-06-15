use futures::future::{AbortHandle, Abortable};
use gloo_timers::future::sleep;
use leptos::*;
use price_chart_wasm::app::stream_abort_handle;
use std::time::Duration;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn aborts_previous_stream() {
    let (handle, reg) = AbortHandle::new_pair();
    stream_abort_handle().set(Some(handle.clone()));
    let fut = Abortable::new(sleep(Duration::from_millis(50)), reg);
    handle.abort();
    assert!(fut.await.is_err());
}
