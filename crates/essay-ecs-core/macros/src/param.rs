use proc_macro::{self};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, 
    token::{Comma, PathSep}, DataStruct, DeriveInput, Fields, Generics, Ident, Index, Type, TypeParam
};
use quote::{__private::TokenStream, quote};


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
                    // var: format_ident!("f_{index}"),
                    // var: index,
                    var: Index::from(index),
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

    let ty_impl_w1 = impl_def_generics(&ast.generics);
    let ty_gen_w1 = impl_use_generics(&ast.generics);
    let ty_arg_w1 = arg_generics(&ast.generics);

    let ty_arg = if ty_arg_w1.len() > 0 {
        quote!{ #ident <#(#ty_arg_w1)*>}
    } else {
        quote!{ #ident }
    };

    let state_types = state_types(&fields);
    let state_init = state_init(&fields);

    // let arg_types = arg_types(&fields);
    let arg_fields = arg_fields(&fields);
    
    //return syn::Error::new(span, format!("Test {:#?}", state_init)).into_compile_error().into();

    quote! {
        impl <#(#ty_impl_w1)*> essay_ecs::core::param::Param for #ident <#(#ty_gen_w1)*> {
            type State = (#(#state_types)*);
            type Arg<'__w, '__s> = #ty_arg;

            fn init(
                meta: &mut essay_ecs::core::schedule::SystemMeta, 
                store: &mut essay_ecs::core::store::Store
            ) -> essay_ecs::core::error::Result<Self::State> {
                Ok((
                    #(#state_init)*
                ))
            }

            fn arg<'__w, '__s>(
                store: &'__w essay_ecs::core::schedule::UnsafeStore,
                state: &'__s mut Self::State, 
            ) -> essay_ecs::core::error::Result<Self::Arg<'__w, '__s>> {
                Ok(#ident {
                    #(#arg_fields)*
                })
            }
        };
    }.into()
}

fn state_types(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField{ty, ..} = field;

        // State doesn't have a lifetime
        //let ty = strip_type_lifetime(ty, Some("__"));

        quote! { <#ty as essay_ecs::core::param::Param>::State, }
    }).collect()
}

fn state_init(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField{ty, ..} = field;

        //let ty = strip_type_lifetime(ty, None);

        quote! { <#ty as essay_ecs::core::param::Param>::init(meta, store)?, }
    }).collect()
}

fn arg_fields(fields: &Vec<ParamField>) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let ParamField { ident, var, ty } = field;

        let ty = strip_type_lifetime(ty, Some("__"));
        
        quote! { #ident: <#ty as essay_ecs::core::param::Param>::arg(store, &mut state . #var)?, }
    }).collect()
}

fn impl_def_generics(gen: &Generics) -> Vec<TokenStream> {
    gen.params.iter().map(|g| {
        match g {
            // syn::GenericParam::Lifetime(_) => quote! { },
            syn::GenericParam::Lifetime(ty) => {
                /*
                if ty.lifetime.ident == "w" {
                    quote! { '__w, }
                } else if ty.lifetime.ident == "s" {
                    quote! { '__s, }
                } else {
                    quote! { #ty, }
                }
                */
                quote! { #ty, }
            },
            syn::GenericParam::Type(ty) => quote! { #ty, },
            syn::GenericParam::Const(ty) => quote! { #ty, },
        }
    }).collect()
}

fn impl_use_generics(gen: &Generics) -> Vec<TokenStream> {
    gen.params.iter().map(|g| {
        match g {
            // syn::GenericParam::Lifetime(_) => quote! {'_, },
            syn::GenericParam::Lifetime(ty) => {
                /*
                if ty.lifetime.ident == "w" {
                    quote! { '__w, }
                } else if ty.lifetime.ident == "s" {
                    quote! { '__s, }
                } else {
                    quote! { #ty, }
                }
                */
                quote! { #ty, }
            }
            syn::GenericParam::Type(TypeParam { ident: id, .. }) => quote! { #id, },
            syn::GenericParam::Const(ty) => quote! { #ty, },
        }
    }).collect()
}

fn arg_generics(gen: &Generics) -> Vec<TokenStream> {
    gen.params.iter().map(|g| {
        match g {
            syn::GenericParam::Lifetime(lifetime) => {
                let mut lifetime = lifetime.clone();
                let ident = &lifetime.lifetime.ident;
                let ident = syn::Ident::new(&format!("__{}", ident), ident.span());
                lifetime.lifetime.ident = ident;

                quote! { #lifetime, }
            }
            syn::GenericParam::Type(TypeParam { ident: id, .. }) => quote! { #id, },
            syn::GenericParam::Const(ty) => quote! { #ty, },
        }
    }).collect()
}

fn strip_type_lifetime(ty: &Type, replace: Option<&str>) -> Type {
    match ty {
        Type::Path(type_path) => {
            let syn::TypePath {
                qself,
                path: syn::Path {
                    leading_colon,
                    segments,
                }
            }: syn::TypePath = type_path.clone();

            Type::Path(syn::TypePath {
                qself,
                path: syn::Path {
                    leading_colon,
                    segments: strip_path(segments, replace),
                }
            })
        }
        Type::Array(type_array) => {
            Type::Array(type_array.clone())
        }
        Type::BareFn(type_bare_fn) => {
            Type::BareFn(type_bare_fn.clone())
        }
        Type::Group(type_group) => {
            Type::Group(type_group.clone())
        }
        Type::ImplTrait(type_impl_trait) => {
            Type::ImplTrait(type_impl_trait.clone())
        }
        Type::Infer(type_infer) => {
            Type::Infer(type_infer.clone())
        }
        Type::Macro(type_macro) => {
            Type::Macro(type_macro.clone())
        }
        Type::Never(type_never) => {
            Type::Never(type_never.clone())
        }
        Type::Paren(type_paren) => {
            Type::Paren(type_paren.clone())
        }
        Type::Ptr(type_ptr) => {
            Type::Ptr(type_ptr.clone())
        }
        Type::Reference(type_reference) => {
            Type::Reference(type_reference.clone())
        }
        Type::Slice(type_slice) => {
            Type::Slice(type_slice.clone())
        }
        Type::TraitObject(type_trait_object) => {
            Type::TraitObject(type_trait_object.clone())
        }
        Type::Tuple(type_tuple) => {
            Type::Tuple(type_tuple.clone())
        }
        Type::Verbatim(token_stream) => {
            Type::Verbatim(token_stream.clone())
        }
        _ => ty.clone(),
    }
}

fn strip_path(
    path: Punctuated<syn::PathSegment, PathSep>,
    replace: Option<&str>,
) -> Punctuated<syn::PathSegment, PathSep> {
    let mut strip_path = Punctuated::new();

    for syn::PathSegment { ident, arguments } in path.iter() {
        let arguments = match arguments {
            syn::PathArguments::AngleBracketed(gen_arguments) => {
                let mut gen_arguments = gen_arguments.clone();
                gen_arguments.args = strip_gen_path(gen_arguments.args, replace);

                syn::PathArguments::AngleBracketed(gen_arguments)
            },
            syn::PathArguments::None => arguments.clone(),
            syn::PathArguments::Parenthesized(_) => arguments.clone(),
        };

        strip_path.push(syn::PathSegment {
            ident: ident.clone(),
            arguments: arguments,
        });
    }

    strip_path
}
fn strip_gen_path(
    path: Punctuated<syn::GenericArgument, Comma>,
    replace: Option<&str>,
) -> Punctuated<syn::GenericArgument, Comma> {
    let mut strip_path = Punctuated::new();

    for arg in path.iter() {
        match arg {
            syn::GenericArgument::Lifetime(syn::Lifetime {
                apostrophe,
                ident,
            }) => {
                if let Some(replace) = replace {
                    // todo: check for 's and 'w
                    strip_path.push(syn::GenericArgument::Lifetime(syn::Lifetime {
                        apostrophe: apostrophe.clone(),
                        ident: syn::Ident::new(&format!("{}{}", replace, ident), ident.span()),
                    }));
                }
            }
            syn::GenericArgument::Type(ty) => {
                strip_path.push(syn::GenericArgument::Type(strip_type_lifetime(ty, replace)));
            }
            _ => {
                strip_path.push(arg.clone());
            }
        };
    }

    strip_path
}

struct ParamField {
    ident: Option<Ident>,
    var: Index, // Ident,
    ty: Type,
}

