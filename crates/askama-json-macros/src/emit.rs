use json::Number;
use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Generics, Ident, ItemStruct, LitBool, LitStr, parse_quote};

pub(crate) fn emit(template: json::Value, input: &ItemStruct) -> TokenStream {
    let mut ctx = EmitContext::new(input.ident.clone());
    let json_template_impl = emit_template_impl(template, input, &mut ctx);
    let askama_templates = emit_askama_templates(ctx, input);
    quote! {
        #json_template_impl
        #askama_templates
    }
}

pub(crate) fn emit_include_bytes(path: &str, span: Span) -> TokenStream {
    let path = LitStr::new(path, span);
    quote! {
        const _: &[::core::primitive::u8] = ::core::include_bytes!(#path);
    }
}

fn emit_template_impl(
    template: json::Value,
    input: &ItemStruct,
    ctx: &mut EmitContext,
) -> TokenStream {
    let template = emit_template(template, ctx);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ident = &input.ident;
    quote! {
        impl #impl_generics ::askama_json::JsonTemplate for #ident #ty_generics #where_clause {
            fn render_with_values(&self, values: &dyn ::askama_json::askama::Values) -> askama_json::askama::Result<::askama_json::serde_json::Value> {
                Ok(#template)
            }
        }
    }
}

fn emit_template(template: json::Value, ctx: &mut EmitContext) -> TokenStream {
    use json::Value::*;
    match template {
        Null => quote! { ::serde_json::Value::Null },
        Bool(v) => emit_bool(v),
        Number(v) => emit_number(v),
        String(s) => {
            let template_name = ctx.add_askama_template(s);
            quote! {
                ::serde_json::Value::String(
                    ::askama_json::askama::Template::render_with_values(
                        &#template_name(self),
                        values
                    )?
                )
            }
        }
        Array(values) => {
            let items: TokenStream = values
                .into_iter()
                .map(|v| emit_template(v, ctx))
                .map(|t| quote!(#t,))
                .collect();
            quote! { ::serde_json::Value::Array(vec![#items]) }
        }
        Object(map) => {
            let entries: TokenStream = map
                .into_iter()
                .map(|(key, value)| (key, emit_template(value, ctx)))
                .map(|(key, value)| {
                    let key = LitStr::new(&key, Span::call_site());
                    quote!((#key.to_string(), #value),)
                })
                .collect();
            quote! { ::serde_json::Value::Object([#entries].into_iter().collect()) }
        }
    }
}

fn emit_bool(value: bool) -> TokenStream {
    let v = LitBool::new(value, Span::call_site());
    quote! { ::serde_json::Value::Bool(#v) }
}

fn emit_number(number: Number) -> TokenStream {
    let json = quote! { ::askama_json::serde_json };
    let value = if let Some(n) = number.as_u64() {
        let lit = Literal::u64_suffixed(n);
        quote! { #json::Number::from(#lit) }
    } else if let Some(n) = number.as_i64() {
        let lit = Literal::i64_suffixed(n);
        quote! { #json::Number::from(#lit) }
    } else if let Some(n) = number.as_f64() {
        let lit = Literal::f64_suffixed(n);
        quote! { #json::Number::from_f64(#lit).unwrap() }
    } else if let Some(n) = number.as_u128() {
        let lit = Literal::u128_suffixed(n);
        quote! { #json::Number::from(#lit) }
    } else if let Some(n) = number.as_i128() {
        let lit = Literal::i128_suffixed(n);
        quote! { #json::Number::from(#lit) }
    } else {
        let v = LitStr::new(&json::to_string(&number).unwrap(), Span::call_site());
        quote! { ::serde_json::from_str(#v).unwrap() }
    };
    quote! { ::serde_json::Value::Number(#value) }
}

fn emit_askama_templates(ctx: EmitContext, input: &ItemStruct) -> TokenStream {
    ctx.into_askama_templates()
        .into_iter()
        .map(|(ident, template)| emit_askama_template(ident, template, input))
        .collect()
}

fn emit_askama_template(ident: Ident, template: String, input: &ItemStruct) -> TokenStream {
    let template_ident = &input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let wrapper_generics = add_template_lifetime(&input.generics);
    let (impl_generics, wrapper_ty_generics, where_clause) = wrapper_generics.split_for_impl();
    let template = LitStr::new(&template, Span::call_site());
    quote! {
        #[derive(::askama_json::askama::Template)]
        #[template(source = #template, ext = "txt", askama = ::askama_json::askama)]
        struct #ident #wrapper_ty_generics (&'__template #template_ident #ty_generics) #where_clause;

        impl #impl_generics ::std::ops::Deref for #ident #wrapper_ty_generics #where_clause {
            type Target = #template_ident #ty_generics;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    }
}

fn add_template_lifetime(generics: &Generics) -> Generics {
    let mut generics = generics.clone();
    generics.lt_token.get_or_insert_with(|| parse_quote!(<));
    generics.gt_token.get_or_insert_with(|| parse_quote!(>));
    generics.params.insert(0, parse_quote!('__template));
    generics
}

struct EmitContext {
    template_name: Ident,
    templates: Vec<(Ident, String)>,
}

impl EmitContext {
    fn new(template_name: Ident) -> Self {
        Self {
            template_name,
            templates: Vec::default(),
        }
    }

    fn add_askama_template(&mut self, template: String) -> Ident {
        let disambiguator = self.templates.len();
        let template_name = format!("{}{}", self.template_name, disambiguator);
        let template_name = Ident::new(&template_name, self.template_name.span());
        self.templates.push((template_name.clone(), template));
        template_name
    }

    fn into_askama_templates(self) -> Vec<(Ident, String)> {
        self.templates
    }
}
