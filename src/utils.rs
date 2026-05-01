/// Sets the `console_error_panic_hook` for better error messages in wasm.
///
/// When the `console_error_panic_hook` feature is enabled, we can get much
/// better error messages in the browser console if our code ever panics.
///
/// For more details see:
/// <https://github.com/rustwasm/console_error_panic_hook#readme>
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
