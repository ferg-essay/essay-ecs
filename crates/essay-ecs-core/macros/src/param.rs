use proc_macro::{self};
use syn::{
    parse_macro_input, spanned::Spanned, DataStruct, DeriveInput, Fields, Generics, Ident, Type
};
use quote::{__private::TokenStream, format_ident, quote};


pub fn derive_param(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let (_, ty_gen, _) = ast.generics.split_for_impl();

    let span = ast.span();

    let DataStruct {
        fields,
        ..
    } = match ast.data {
        syn::Data::Struct(data) => data,
        syn::Data::Enum(_) => {
            todo!("enum not supported for derive(Param)")
        },
        syn::Data::Union(_) => todo!("union not supported for derive(Param)"),
    };

    let fields: Vec<ParamField> = match fields {
        Fields::Named(ref fields) => {
            fields.named.iter().enumerate().map(|(index, field)| {
                ParamField {
                    ident: field.ident.clone(),
                    var: format_ident!("f_{index}"),
                    ty: field.ty.clone(),
                }
            }).collect()
        }
        Fields::Unnamed(_) => {
            return syn::Error::new(span, "tuples currently unsupported").into_compile_error().into();
        },
        Fields::Unit => {
            //Vec::new()
            return syn::Error::new(span, "unit field unknown").into_compile_error().into();
        }
    };

    let ident = ast.ident.clone();
    // let generics = ast.generics;

    let ty_impl_w1 = strip_generics(&ast.generics);
    let ty_gen_w1 = rename_generics(&ast.generics);

    let state_types = state_types(&fields);
    let state_init = state_init(&fields);

    // let arg_types = arg_types(&fields);
    let arg_fields = arg_fields(&fields);
    
    //return syn::Error::new(span, format!("Test {:#?}", state_init)).into_compile_error().into();

    quote! {
        const _: () = {
            struct __State<'w, 's> {
                #(#state_types)*
                marker: PhantomData<(&'w u8, &'s u8)>,
            }

            fn __state_init<'w, 's>(
                meta: &mut essay_ecs::core::schedule::SystemMeta,
                store: &mut essay_ecs::core::store::Store
            ) -> essay_ecs::core::error::Result<__State<'w, 's>> {
                Ok(__State {
                    #(#state_init)*
                    marker: PhantomData::default(),
                })
            }

            impl <#(#ty_impl_w1)*> essay_ecs::core::param::Param for #ident <#(#ty_gen_w1)*> {
                type State = __State<'static, 'static>;
                type Arg<'w, 's> = #ident #ty_gen;

                fn init(
                    meta: &mut essay_ecs::core::schedule::SystemMeta, 
                    store: &mut essay_ecs::core::store::Store
                ) -> essay_ecs::core::error::Result<Self::State> {
                    __state_init(meta, store)
                }

                fn arg<'w, 's>(
                    store: &'w essay_ecs::core::schedule::UnsafeStore,
                    state: &'s mut Self::State, 
                ) -> essay_ecs::core::error::Result<Self::Arg<'w, 's>> {
                    Ok(#ident {
                        #(#arg_fields)*
                    })
                }
            }
        };
    }.into()
}

fn rename_generics(gen: &Generics) -> Vec<TokenStream> {
    gen.params.iter().map(|g| {
        match g {
            syn::GenericParam::Lifetime(_) => quote! {'_, },
            syn::GenericParam::Type(ty) => quote! {#ty, },
            syn::GenericParam::Const(ty) => quote! { #ty, },
        }
    }).collect()
}

fn strip_generics(gen: &Generics) -> Vec<TokenStream> {
    gen.params.iter().map(|g| {
        match g {
            syn::GenericParam::Lifetime(_) => quote! { },
            syn::GenericParam::Type(ty) => quote! {#ty, },
            syn::GenericParam::Const(ty) => quote! { #ty, },
        }
    }).collect()
}

struct ParamField {
    ident: Option<Ident>,
    var: Ident,
    ty: Type,
}

fn state_types(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField{var, ty, ..} = field;

        quote! { #var: <#ty as essay_ecs::core::param::Param>::State, }
    }).collect()
}

fn state_init(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField{var, ty, ..} = field;

        quote! { #var: <#ty as essay_ecs::core::param::Param>::State::init(meta, store)?, }
    }).collect()
}

fn arg_fields(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField { ident, var, ty } = field;
        
        quote! { #ident: <#ty as essay_ecs::core::param::Param>::Arg::<'w, 's>::arg(store, &mut state.#var)?, }
    }).collect()
}

