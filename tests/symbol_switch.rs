use leptos::*;
use price_chart_wasm::domain::market_data::Symbol;
use price_chart_wasm::global_state::globals;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn updates_current_symbol() {
    let g = globals();
    assert_eq!(g.current_symbol.get().value(), "BTCUSDT");
    g.current_symbol.set(Symbol::from("ETHUSDT"));
    assert_eq!(g.current_symbol.get().value(), "ETHUSDT");
}
