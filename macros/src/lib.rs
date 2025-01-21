use proc_macro2::TokenStream;
use syn::parse_macro_input;

use quote::quote;

extern crate proc_macro;



#[proc_macro_derive(ShaderStruct, attributes(wgsl))]
pub fn derive_shader_struct(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let input = parse_macro_input!(input as syn::DeriveInput);

    let syn::Data::Struct(strct) = &input.data else {
        return syn::Error::new_spanned(input, "only structs are supported").to_compile_error().into()
    };

    let strct_ty = &input.ident;

    let mut fields = quote!();

    for field in &strct.fields {
        let ty = &field.ty;
        let name = field.ident.clone().expect("only named fields supported").to_string();

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

    let strct_impl = quote! {
        impl atlas::gpu::ShaderStruct for #strct_ty {
            const FIELDS: &'static [(&'static str, atlas::gpu::Primitive)] = &[#fields];
        }
    };


    strct_impl.into()
}
