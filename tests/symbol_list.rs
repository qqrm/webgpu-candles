use price_chart_wasm::domain::market_data::value_objects::{Symbol, default_symbols};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn returns_three_symbols() {
    let symbols = default_symbols();
    assert_eq!(symbols.len(), 3);
    assert!(symbols.contains(&Symbol::from("BTCUSDT")));
    assert!(symbols.contains(&Symbol::from("ETHUSDT")));
    assert!(symbols.contains(&Symbol::from("SOLUSDT")));
}
