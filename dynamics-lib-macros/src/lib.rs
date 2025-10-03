use proc_macro::TokenStream;

mod validate;
mod utils;

/// Derive macro for validation framework
///
/// # Example
/// ```rust
/// #[derive(Validate)]
/// struct CreateForm {
///     #[validate(not_empty, message = "Name required")]
///     name: TextInputField,
///
///     #[validate(required, message = "Source required")]
///     source: SelectField,
/// }
/// ```
#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    validate::derive(input)
}

// Future proc macros:
// - #[derive(App)] - Widget auto-routing
// - Field type generators
