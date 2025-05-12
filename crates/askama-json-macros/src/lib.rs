extern crate proc_macro;

use emit::{emit, emit_include_bytes};
use error::Error;
use parse::parse_input;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

mod emit;
mod error;
mod parse;

#[proc_macro_derive(JsonTemplate, attributes(json_template))]
pub fn derive_json_template(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    match derive_json_template_impl(input) {
        Ok(s) => s.into(),
        Err(e) => e.emit_as_item_tokens().into(),
    }
}

fn derive_json_template_impl(input: ItemStruct) -> Result<TokenStream2, Error> {
    let derive_input = parse_input(&input)?;
    let template_impl = emit(derive_input.template, &input);
    let include_bytes = emit_include_bytes(&derive_input.path.0, derive_input.path.1);
    Ok(quote! {
        #template_impl
        #include_bytes
    })
}
