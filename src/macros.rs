/// Helper macro to define functions returning global signals.
/// Usage: `global_signal!(fn_name, field: Type);`
#[macro_export]
macro_rules! global_signal {
    ($vis:vis $name:ident, $field:ident : $ty:ty) => {
        $vis fn $name() -> ::leptos::RwSignal<$ty> {
            $crate::global_state::globals().$field
        }
    };
}
