use leptos::ev::EventDescriptor;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{AddEventListenerOptions, Event};

#[derive(Clone, Debug)]
pub struct EventOptions {
    pub passive: bool,
    pub capture: bool,
    pub once: bool,
}

impl Default for EventOptions {
    fn default() -> Self {
        Self { passive: true, capture: false, once: false }
    }
}

pub struct WindowEventListenerHandle {
    event_name: String,
    callback: Closure<dyn FnMut(Event)>,
    capture: bool,
}

impl WindowEventListenerHandle {
    pub fn remove(self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback_and_bool(
                &self.event_name,
                self.callback.as_ref().unchecked_ref(),
                self.capture,
            );
        }
    }
}

pub fn window_event_listener_with_options<E>(
    event: E,
    options: &EventOptions,
    mut cb: impl FnMut(E::EventType) + 'static,
) -> WindowEventListenerHandle
where
    E: EventDescriptor + 'static,
    E::EventType: JsCast,
{
    let opts = AddEventListenerOptions::new();
    opts.set_passive(options.passive);
    opts.set_capture(options.capture);
    opts.set_once(options.once);

    let event_name = event.name().into_owned();
    let callback = Closure::wrap(Box::new(move |ev: Event| {
        cb(ev.unchecked_into::<E::EventType>());
    }) as Box<dyn FnMut(Event)>);

    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback_and_add_event_listener_options(
            &event_name,
            callback.as_ref().unchecked_ref(),
            &opts,
        );
    }

    WindowEventListenerHandle { event_name, callback, capture: options.capture }
}
use leptos::{HtmlElement, html::AnyElement};

pub fn wheel_event_options(_el: HtmlElement<AnyElement>, _opts: &EventOptions) {}
