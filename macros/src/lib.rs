use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{Ident, Token, parse_macro_input};

use quote::{format_ident, quote};

extern crate proc_macro;

// struct JITFnAttrib {
//     import_name: Option<Ident>,
//     extrn: bool,
// }

// impl Parse for JITFnAttrib {
//     fn parse(input: ParseStream) -> Result<Self> {
//         if let Ok(_) = input.parse::<Token![extern]>() {
//             let import_name = if let Ok(_) = input.parse::<Token![:]>() {
//                 let name: syn::Ident = input.parse()?;
//                 Some(name)
//             } else {
//                 None
//             };

//             Ok(Self {
//                 import_name,
//                 extrn: true
//             })
//         } else if let Ok(_) = input.parse::<syn::parse::Nothing>() {
//             Ok(Self {
//                 import_name: None,
//                 extrn: false,
//             })
//         } else {
//             Err(syn::Error::new(input.span(), "invalid attribute"))
//         }
//     }
// }

#[proc_macro_attribute]
pub fn jit_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut res = quote! {
        #[unsafe(no_mangle)]
    };

    res.extend(TokenStream2::from(item));
    res.into()
}

// #[proc_macro_attribute]
// pub fn jit_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let fn_attrib = parse_macro_input!(attr as JITFnAttrib);

//     let sig = if fn_attrib.extrn {
//         let sig = parse_macro_input!(item as syn::Signature);
//         sig
//     } else {
//         let fn_item = parse_macro_input!(item as syn::ItemFn);
//         fn_item.sig
//     };

//     let import_fn_name = match fn_attrib.import_name {
//         Some(name) => name,
//         None => sig.ident.clone(),
//     };

//     let mut params = vec![];
//     let span = sig.span();
//     for arg in sig.inputs {
//         let syn::FnArg::Typed(arg) = arg else {
//             return syn::Error::new(span, "self parameter is not allowed").to_compile_error().into();
//         };
//         params.push(arg.ty)
//     }

//     let rs_name = sig.ident;
//     quote! {
//         #[allow(non_camel_case_types)]
//         struct #rs_name;

//     }.into()
// }

#[proc_macro_derive(ShaderStruct, attributes(wgsl))]
pub fn derive_shader_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let syn::Data::Struct(strct) = &input.data else {
        return syn::Error::new_spanned(input, "only structs are supported")
            .to_compile_error()
            .into();
    };

    let strct_ty = &input.ident;

    let mut fields = quote!();

    for field in &strct.fields {
        let ty = &field.ty;
        let name = field
            .ident
            .clone()
            .expect("only named fields supported")
            .to_string();

        let mut attrib = String::new();

        for attr in &field.attrs {
            if attr.path().is_ident("wgsl") {
                if let syn::Meta::List(meta_list) = &attr.meta {
                    attrib = meta_list.tokens.to_string();
                }
            }
        }

        let wgsl_field = format!("{attrib} {name}");

        fields.extend(quote! {
            (#wgsl_field, <#ty as atlas::gpu::GpuPrimitive>::GPU_PRIMITIVE),
        });
    }

    for attr in &input.attrs {
        if attr.path().is_ident("wgsl") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let wgsl_builtin = meta_list.tokens.to_string();

                let (field, ty_str) = wgsl_builtin.rsplit_once(':').unwrap();
                let ty = Ident::new(ty_str.trim(), attr.bracket_token.span.open());

                fields.extend(quote! {
                    (#field, <#ty as atlas::gpu::GpuPrimitive>::GPU_PRIMITIVE),
                });
            }
        }
    }

    let strct_impl = quote! {
        impl atlas::gpu::ShaderStruct for #strct_ty {
            const FIELDS: &'static [(&'static str, atlas::gpu::Primitive)] = &[#fields];
        }
    };

    strct_impl.into()
}

#[derive(Debug)]
enum AstNode {
    Variable(Ident),
    Literal(f64),
    UnaryOp {
        op: Ident,
        expr: Box<AstNode>,
    },
    BinaryOp {
        op: Ident,
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
    },
    FunctionCall {
        func: Ident,
        arg: Box<AstNode>,
    },
}

fn parse_expr(input: ParseStream, min_precedence: u8) -> Result<AstNode> {
    let mut lhs = parse_primary(input)?;
    loop {
        let op_info = match get_operator(input) {
            Some(info) => info,
            None => break,
        };
        let (op_str, precedence) = op_info;
        if precedence < min_precedence {
            break;
        }

        // Parse the specific token based on detected operator
        match op_str {
            "POW" => {
                input.parse::<Token![^]>()?;
            }
            "MUL" => {
                input.parse::<Token![*]>()?;
            }
            "DIV" => {
                input.parse::<Token![/]>()?;
            }
            "ADD" => {
                input.parse::<Token![+]>()?;
            }
            "SUB" => {
                input.parse::<Token![-]>()?;
            }
            _ => unreachable!("Unknown operator"),
        };

        let next_min = if precedence == 4 {
            precedence
        } else {
            precedence + 1
        };
        let rhs = parse_expr(input, next_min)?;
        lhs = AstNode::BinaryOp {
            op: Ident::new(op_str, proc_macro2::Span::call_site()),
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }
    Ok(lhs)
}

// fn parse_expr(input: ParseStream, min_precedence: u8) -> Result<AstNode> {
//     let mut lhs = parse_primary(input)?;
//     loop {
//         let op_info = match get_operator(input) {
//             Some(info) => info,
//             None => break,
//         };
//         let (op_str, precedence) = op_info;
//         if precedence < min_precedence {
//             break;
//         }
//         input.parse::<Token>()?; // Consume the operator token

//         let next_min = if precedence == 4 { precedence } else { precedence + 1 };
//         let rhs = parse_expr(input, next_min)?;
//         lhs = AstNode::BinaryOp {
//             op: Ident::new(op_str, proc_macro2::Span::call_site()),
//             lhs: Box::new(lhs),
//             rhs: Box::new(rhs),
//         };
//     }
//     Ok(lhs)
// }

fn get_operator(input: ParseStream) -> Option<(&'static str, u8)> {
    if input.peek(Token![^]) {
        Some(("POW", 4))
    } else if input.peek(Token![*]) {
        Some(("MUL", 3))
    } else if input.peek(Token![/]) {
        Some(("DIV", 3))
    } else if input.peek(Token![+]) {
        Some(("ADD", 2))
    } else if input.peek(Token![-]) {
        Some(("SUB", 2))
    } else {
        None
    }
}

fn parse_primary(input: ParseStream) -> Result<AstNode> {
    if input.peek(Token![-]) {
        let _ = input.parse::<Token![-]>()?;
        let expr = parse_primary(input)?;
        return Ok(AstNode::UnaryOp {
            op: Ident::new("NEG", proc_macro2::Span::call_site()),
            expr: Box::new(expr),
        });
    } else if input.peek(syn::Ident) {
        let ident: Ident = input.parse()?;
        if ident == "sin" || ident == "cos" || ident == "tan" || ident == "exp" {
            let content;
            syn::parenthesized!(content in input);
            let arg = content.parse::<AstNode>()?;
            return Ok(AstNode::FunctionCall {
                func: ident,
                arg: Box::new(arg),
            });
        } else if ident == "x" || ident == "y" {
            return Ok(AstNode::Variable(ident));
        } else {
            return Err(input.error("unknown function or variable"));
        }
    } else if input.peek(syn::LitInt) {
        let lit = input.parse::<syn::LitInt>()?;
        let value = lit.base10_parse::<f64>()?;
        return Ok(AstNode::Literal(value));
    } else if input.peek(syn::LitFloat) {
        let lit = input.parse::<syn::LitFloat>()?;
        let value = lit.base10_parse()?;
        return Ok(AstNode::Literal(value));
    } else if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let expr = content.parse::<AstNode>()?;
        return Ok(expr);
    }
    Err(input.error("expected primary expression"))
}

impl Parse for AstNode {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_expr(input, 0)
    }
}

#[proc_macro]
pub fn implicit_fn(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as AstNode);
    let mut code_gen = CodeGenerator::new();
    code_gen.generate(&ast, 1);
    let bytecode = code_gen.into_bytecode();
    let expanded = quote! {
        [
            #(#bytecode),*,
            op::EXT(0)
        ]
    };
    expanded.into()
}

struct CodeGenerator {
    instructions: Vec<proc_macro2::TokenStream>,
    next_register: u8,
}

impl CodeGenerator {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            next_register: 3,
        }
    }

    fn into_bytecode(self) -> Vec<proc_macro2::TokenStream> {
        self.instructions
    }

    fn generate(&mut self, node: &AstNode, target_reg: u8) {
        match node {
            AstNode::Variable(ident) => {
                let reg = if ident == "x" { 1 } else { 2 };
                if reg != target_reg {
                    self.emit_move(reg, target_reg);
                }
            }
            AstNode::Literal(value) => {
                self.emit_unary_imm(*value, target_reg);
            }
            AstNode::UnaryOp { op, expr } => {
                self.generate(expr, target_reg);
                self.emit_unary_op(&op.to_string(), target_reg);
            }
            AstNode::BinaryOp { op, lhs, rhs } => {
                self.generate_binary_op(&op.to_string(), lhs, rhs, target_reg);
            }
            AstNode::FunctionCall { func, arg } => {
                self.generate(arg, target_reg);
                self.emit_unary_op(&func.to_string().to_uppercase(), target_reg);
            }
        }
    }

    fn generate_binary_op(&mut self, op: &str, lhs: &AstNode, rhs: &AstNode, target_reg: u8) {
        let temp_reg = self.next_register;
        self.next_register += 1;
        self.generate(lhs, temp_reg);
        self.generate(rhs, target_reg);
        self.emit_binary_op(op, temp_reg, target_reg, target_reg);
        self.next_register -= 1;
    }

    fn emit_unary_op(&mut self, op: &str, reg: u8) {
        let op_ident = match op {
            "NEG" => quote!(op::NEG),
            "SIN" => quote!(op::SIN),
            "COS" => quote!(op::COS),
            "TAN" => quote!(op::TAN),
            "EXP" => quote!(op::EXP),
            _ => panic!("Unsupported unary operator: {}", op),
        };
        self.instructions.push(quote! {
            #op_ident(#reg, #reg)
        });
    }

    fn emit_unary_imm(&mut self, value: f64, reg: u8) {
        self.instructions.push(quote! {
            op::MOV_IMM(#value, #reg)
        });
    }

    fn emit_binary_op(&mut self, op: &str, lhs_reg: u8, rhs_reg: u8, target_reg: u8) {
        let (op_type, lhs_type, rhs_type) = match op {
            "ADD" => ("ADD", "LHS", "RHS"),
            "SUB" => ("SUB", "LHS", "RHS"),
            "MUL" => ("MUL", "LHS", "RHS"),
            "DIV" => ("DIV", "LHS", "RHS"),
            "POW" => ("POW", "LHS", "RHS"),
            _ => panic!("Unsupported binary operator: {}", op),
        };
        let op_ident = format_ident!("{}_{}_{}", op_type, lhs_type, rhs_type);
        self.instructions.push(quote! {
            op::#op_ident(#lhs_reg, #rhs_reg, #target_reg)
        });
    }

    fn emit_move(&mut self, from_reg: u8, to_reg: u8) {
        self.instructions.push(quote! {
            op::MOV(#from_reg, #to_reg)
        });
    }
}
