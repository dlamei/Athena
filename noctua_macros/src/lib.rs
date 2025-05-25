use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitInt, Token,
    parse::{Parse, ParseStream},
};

#[proc_macro]
pub fn noctua(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as ExprMacro);
    parsed.into_token_stream().into()
}

struct ExprMacro(proc_macro2::TokenStream);

impl ExprMacro {
    fn into_token_stream(self) -> proc_macro2::TokenStream {
        self.0
    }
}

impl Parse for ExprMacro {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = parse_add_sub(input)?;
        Ok(ExprMacro(expr))
    }
}

fn parse_add_sub(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    let mut node = parse_mul_div(input)?;
    while input.peek(Token![+]) || input.peek(Token![-]) {
        let op: proc_macro2::TokenTree = input.parse()?;
        let right = parse_mul_div(input)?;
        let op_ts = if op.to_string() == "+" {
            quote! { + }
        } else {
            quote! { - }
        };
        node = quote! { ( #node #op_ts #right ) };
    }
    Ok(node)
}

fn parse_mul_div(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    let mut node = parse_pow(input)?;
    while input.peek(Token![*]) || input.peek(Token![/]) {
        let op: proc_macro2::TokenTree = input.parse::<proc_macro2::TokenTree>()?;
        let right = parse_pow(input)?;
        let op_ts = if op.to_string() == "*" {
            quote! { * }
        } else {
            quote! { / }
        };
        node = quote! { ( #node #op_ts #right ) };
    }
    Ok(node)
}

fn parse_pow(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    let base = parse_primary(input)?;
    if input.peek(Token![^]) {
        let _: Token![^] = input.parse()?;
        let exponent = parse_pow(input)?;
        let ts = quote! {
            Expr::pow(#base, #exponent)
        };
        Ok(ts)
    } else {
        Ok(base)
    }
}

fn parse_primary(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    if input.peek(LitInt) {
        let lit: LitInt = input.parse()?;
        let val = lit.base10_parse::<u32>()?;
        Ok(quote! { Expr::u32(#val) })
    } else if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        let name = ident.to_string();
        Ok(quote! { Expr::var(#name) })
    } else if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let inner = parse_add_sub(&content)?;
        Ok(quote! { ( #inner ) })
    } else {
        Err(input.error("Unexpected token in expression"))
    }
}
