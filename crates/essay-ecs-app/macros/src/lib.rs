mod event;
use proc_macro::TokenStream;

extern crate proc_macro;
extern crate syn;
extern crate quote;

#[proc_macro_derive(Event, attributes(component))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    event::derive_event(input)
}
