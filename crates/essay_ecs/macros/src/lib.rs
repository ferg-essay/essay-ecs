mod component;
mod label;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

extern crate proc_macro;
extern crate syn;
extern crate quote;



#[proc_macro_derive(SystemParam, attributes(ticker_param))]
pub fn derive_ticker_param(input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive_component(input)
}

#[proc_macro_derive(ScheduleLabel, attributes(component))]
pub fn derive_schedule_label(input: TokenStream) -> TokenStream {
    label::derive_schedule_label(input)
}

#[proc_macro_derive(Phase, attributes(component))]
pub fn derive_task_set(input: TokenStream) -> TokenStream {
    label::derive_phase(input)
}
