use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

pub fn derive_phase(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;

    TokenStream::from(quote! {
        impl essay_ecs::core::schedule::Phase for #name {
            fn box_clone(&self) -> Box<dyn essay_ecs::core::schedule::Phase> {
                Box::new(self.clone())
            }
        }
    })
}
