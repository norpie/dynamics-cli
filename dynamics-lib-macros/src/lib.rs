use proc_macro::TokenStream;

// Placeholder for future derive macros
// Examples from reducal.md:
// - #[derive(Validate)] - validation framework
// - #[derive(App)] - widget auto-routing
// - Field type generators (TextInputField, SelectField, etc)

#[proc_macro_derive(Placeholder)]
pub fn placeholder_derive(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
