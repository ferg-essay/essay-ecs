use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;
    let (ty_impl, ty_gen, _) = ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #ty_impl essay_ecs::core::entity::Component for #name #ty_gen {

        }
    })
}