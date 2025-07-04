use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, Ident, ItemFn, LitInt, Pat, Token, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro]
pub fn noctua(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as ExprMacro);
    parsed.into_token_stream().into()
}

#[derive(Debug, Clone)]
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
            #base.pow(#exponent)
        };
        Ok(ts)
    } else {
        Ok(base)
    }
}

fn parse_primary(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    if input.peek(LitInt) {
        let lit: LitInt = input.parse()?;
        let val = lit.base10_parse::<i32>()?;
        Ok(quote! { noctua::Expr::i32(#val) })
    } else if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        let name = ident.to_string();

        if input.peek(syn::token::Paren) {
            let fn_call;
            let _ = parenthesized!(fn_call in input);
            let args: syn::Result<Punctuated<ExprMacro, Token![,]>> =
                fn_call.parse_terminated(ExprMacro::parse, Token![,]);
            if let Ok(args) = args {
                let args: Vec<_> = args.into_iter().map(|a| a.0).collect();
                Ok(quote! { noctua::Expr::#ident(#(#args,)*)})
            } else {
                Ok(quote! { noctua::Expr::#name()})
            }
        } else if &name == "undef" {
            Ok(quote! { noctua::Expr::undef() })
        } else {
            Ok(quote! { noctua::Expr::var(#name) })
        }
    } else if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let inner = parse_add_sub(&content)?;
        Ok(quote! { ( #inner ) })
    } else if input.peek(Token![-]) {
        let _: Token![-] = input.parse()?;
        let expr = parse_mul_div(input)?;
        Ok(quote! { std::ops::Neg::neg(#expr) })
    } else {
        Err(input.error("Unexpected token in expression"))
    }
}

#[cfg(feature = "disable_log")]
#[proc_macro_attribute]
pub fn log_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[cfg(not(feature = "disable_log"))]
#[proc_macro_attribute]
pub fn log_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        vis, sig, block, ..
    } = parse_macro_input!(item as ItemFn);

    // Collect parameter identifiers (including `self`)
    let mut has_self = false;
    let params: Vec<_> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => {
                has_self = true;
                Some(format_ident!("self"))
            }
            FnArg::Typed(pat) => {
                if let Pat::Ident(p) = &*pat.pat {
                    Some(p.ident.clone())
                } else {
                    None
                }
            }
        })
        .collect();

    let n_params = params.len();

    // Helper to read a bool value from global config at runtime
    let bool_cfg = |key: &str, default: bool| {
        let key_str = key.to_string();
        quote! {{
            crate::__GLOB_FN_LOG_CFG__
                .iter()
                .find_map(|(k, v)| if *k == #key_str { Some(*v) } else { None })
                .map(|s| s == "true")
                .unwrap_or(#default)
        }}
    };

    let dbg_enabled = bool_cfg("dbg", true);
    let has_return = !matches!(sig.output, syn::ReturnType::Default);

    // Format the parameter list
    let fmt_params = quote! {{
        if #n_params == 0 {
            "".to_string()
        } else {
            let args = vec![
                #(format!("{:?}", &#params)),*
            ];

            let fmt = itertools::Itertools::format(args.into_iter(), ", ");
            format!("( {fmt} )")
        }
    }};

    // Generate function‐name at runtime
    let fmt_name = quote! {{
        fn f() {}
        fn type_name_of<T>(_:T) -> &'static str {
            std::any::type_name::<T>()
        }
        let full_name = type_name_of(f);
        let path = full_name.strip_suffix("::f").unwrap_or(full_name);
        {
            // use itertool::Itertools;
            let path: Vec<_> = path.rsplit("::").take(2).collect::<Vec<_>>().into_iter().rev().collect();
            path.join("::")
            // path.rsplit("::").take(2).collect::<Vec<_>>().into_iter().rev().join("::")
        }
    }};

    let tl = "\u{0250C}";
    let v_bar = "\u{2502}";
    let h_bar = "\u{2500}";
    let pad = "";

    let v_sep = format!("{pad}{v_bar}{pad}");
    // Entry log
    let entry = quote! {{
        let mut bar = (#v_sep).repeat(__log_fn_current_level__);
        if !bar.is_empty() {
            bar += #pad;
        } else {
            bar = #pad.to_string() + &bar;
        }
        log::trace!("{}\u{0250C}{} {}{}", bar, #h_bar, __log_fn_name__, #fmt_params);
    }};

    let ret_fmt = if has_return {
        quote! {{
            // if #dbg_enabled {
            format!("\u{02514}> {:?}", __log_fn_result__)
            // } else {
            //     format!("\u{02514}> {}", __log_fn_result__)
            // }
        }}
    } else if has_self {
        quote! {{
            // if #dbg_enabled {
            format!("\u{02514}: {:?}", self)
            // } else {
            //     format!("\u{02514}: {}", self)
            // }
        }}
    } else {
        quote! { "\u{02514}".to_string() }
    };

    let exit = quote! {{
        let mut bar = (#v_sep).repeat(__log_fn_current_level__);
        if !bar.is_empty() {
            bar += #pad;
        } else {
            bar = #pad.to_string() + &bar;
        }
        if !bar.is_empty() {
            log::trace!("{bar}{}", #ret_fmt);
            log::trace!("{bar}");
        } else {
            log::trace!("{bar}{}\n", #ret_fmt);
        }
        // blank line when unwound fully
        // log::trace!("{}", bar);
    }};

    // let real_fn = format_ident!("__log_fn_{}__", sig.ident);
    // let mut real_fn_sig = sig.clone();
    // real_fn_sig.ident = real_fn.clone();

    // let call_real_fn = if has_self {
    //     quote! {
    //         Self::#real_fn(#(#params),*)
    //     }
    // } else {
    //     quote! {
    //         Self::#real_fn(#(#params),*)
    //     }
    // };

    let expanded = quote! {
        // #[doc(hidden)]
        // #[inline]
        // #real_fn_sig {
        //     #block
        // }

        #vis #sig {
            let __log_fn_name__ = { #fmt_name };
            let __log_fn_current_level__ = crate::__LOG_FN_INDENT__.with(|c| c.get());

            #entry
            crate::__LOG_FN_INDENT__.with(|c| c.set(c.get() + 1));

            // let __log_fn_result__ = #call_real_fn;
            let __log_fn_result__ = (move || {
                #block
            })();

            crate::__LOG_FN_INDENT__.with(|c| c.set(c.get() - 1));
            #exit

            __log_fn_result__
        }

    };

    expanded.into()
}

#[derive(Default, Debug, Clone)]
struct FnLogConfig {
    entries: Vec<(String, String)>,
}

impl Parse for FnLogConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            let val: syn::ExprLit = input.parse()?;

            let _ = input.parse::<Token![,]>();

            let val_str = match val.lit {
                syn::Lit::Str(lit_str) => lit_str.value(),
                syn::Lit::Int(lit_int) => lit_int.base10_digits().to_string(),
                syn::Lit::Bool(lit_bool) => lit_bool.value().to_string(),
                _ => todo!(),
            };
            entries.push((ident.to_string(), format!("{}", val_str)));
        }
        Ok(FnLogConfig { entries })
    }
}

#[cfg(feature = "disable_log")]
#[proc_macro]
pub fn setup_fn_log(input: TokenStream) -> TokenStream {
    // input
    quote! {}.into()
}

/// should be called in the lib.rs file
#[cfg(not(feature = "disable_log"))]
#[proc_macro]
pub fn setup_fn_log(input: TokenStream) -> TokenStream {
    let FnLogConfig { entries } = parse_macro_input!(input as FnLogConfig);

    let pairs = entries.iter().map(|(k, v)| {
        quote! { (#k, #v) }
    });

    let n_pairs = pairs.len();

    quote! {
        #[doc(hidden)]
        pub(crate) const __GLOB_FN_LOG_CFG__: [(&'static str, &'static str); #n_pairs] = [ #(#pairs),* ];

        thread_local! {
            #[doc(hidden)]
            pub(crate) static __LOG_FN_INDENT__: std::cell::Cell<usize> = std::cell::Cell::new(0);
        }
    }.into()
}
