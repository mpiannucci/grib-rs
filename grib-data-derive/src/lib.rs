extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, ItemEnum, DeriveInput};

#[proc_macro_derive(DisplayDescription, attributes(desc))]
pub fn display_description(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let item: Item = input.into();

    if let Item::Enum(e) = item {
        // Build the output, possibly using quasi-quotation
        let expanded = generate_display_impl(&e);

        println!("{}", expanded);

        // Hand the output tokens back to the compiler
        TokenStream::from(expanded)
    } else {
        panic!("Only Enums are supported!");
    }
}

fn generate_display_impl(enum_data: &ItemEnum) -> TokenStream {
    let name: &syn::Ident = &enum_data.ident;
    let variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma> = &enum_data.variants;
    let variant_iter = variants.into_iter().map(|v| v.ident.clone());
    

    (quote! {
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let description = match self {
                    #(
                        #name::#variant_iter => "test",
                    )*
                };
                write!(f, "{}", description)
            }
        }
    }).into()
}