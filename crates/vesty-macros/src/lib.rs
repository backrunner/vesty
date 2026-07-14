use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Fields, Ident, ImplItem, ItemImpl, LitStr, ReturnType, Type,
    parse_macro_input, spanned::Spanned,
};

#[proc_macro_attribute]
pub fn vst3_panic_boundary(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);
    expand_vst3_panic_boundary(item).into()
}

fn expand_vst3_panic_boundary(mut item: ItemImpl) -> TokenStream2 {
    for impl_item in &mut item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        let body = &method.block;
        let fallback = panic_fallback(&method.sig.output);
        method.block = syn::parse_quote!({
            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #body)) {
                ::std::result::Result::Ok(value) => value,
                ::std::result::Result::Err(_) => #fallback,
            }
        });
    }
    quote!(#item)
}

fn panic_fallback(output: &ReturnType) -> TokenStream2 {
    let ReturnType::Type(_, ty) = output else {
        return quote!(());
    };
    if let Type::Path(path) = ty.as_ref()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "tresult")
    {
        quote!(kResultFalse)
    } else {
        quote!(::core::default::Default::default())
    }
}

#[proc_macro_derive(Params, attributes(param))]
pub fn derive_params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_params(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_params(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "Params can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                name.span(),
                "Params can only be derived for structs",
            ));
        }
    };

    let mut param_fields = Vec::new();
    for field in fields {
        let Some(field_name) = field.ident else {
            continue;
        };
        let attrs = parse_param_attrs(&field.attrs)?;
        if attrs.skip {
            continue;
        }
        if !is_supported_param_type(&field.ty) {
            return Err(syn::Error::new_spanned(
                field.ty,
                "Params derive supports FloatParam, BoolParam, and ChoiceParam fields; add #[param(skip)] to ignore this field",
            ));
        }
        if attrs.bypass && !is_bool_param_type(&field.ty) {
            return Err(syn::Error::new_spanned(
                field.ty,
                "#[param(bypass)] can only be used on BoolParam fields",
            ));
        }
        param_fields.push(ParamField {
            ident: field_name,
            id: attrs.id,
            bypass: attrs.bypass,
        });
    }

    let params_path = params_module_path();
    let param_indices = (0..param_fields.len())
        .map(syn::Index::from)
        .collect::<Vec<_>>();
    let spec_exprs = param_fields
        .iter()
        .map(ParamField::spec_expr)
        .collect::<Vec<_>>();
    let id_exprs = param_fields
        .iter()
        .map(ParamField::id_expr)
        .collect::<Vec<_>>();
    let param_idents = param_fields
        .iter()
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_generics #params_path::ParamCollection for #name #ty_generics #where_clause {
            fn specs(&self) -> ::std::vec::Vec<#params_path::ParamSpec> {
                ::std::vec![#(#spec_exprs),*]
            }

            fn get_normalized(&self, id: &str) -> ::std::option::Option<f64> {
                #(
                    if id == #id_exprs {
                        return ::std::option::Option::Some(self.#param_idents.normalized());
                    }
                )*
                ::std::option::Option::None
            }

            fn set_normalized(
                &self,
                id: &str,
                normalized: f64,
            ) -> ::std::result::Result<(), #params_path::ParamError> {
                #(
                    if id == #id_exprs {
                        self.#param_idents.set_normalized(normalized);
                        return ::std::result::Result::Ok(());
                    }
                )*
                ::std::result::Result::Err(#params_path::ParamError::Unknown(id.to_string()))
            }

            fn resolve(&self, id: &str) -> ::std::option::Option<#params_path::ParamHandle> {
                #(
                    if id == #id_exprs {
                        return ::std::option::Option::Some(#params_path::ParamHandle::from_index(#param_indices));
                    }
                )*
                ::std::option::Option::None
            }

            fn get_normalized_by_handle(
                &self,
                handle: #params_path::ParamHandle,
            ) -> ::std::option::Option<f64> {
                match handle.index() {
                    #(
                        #param_indices => ::std::option::Option::Some(self.#param_idents.normalized()),
                    )*
                    _ => ::std::option::Option::None,
                }
            }

            fn set_normalized_by_handle(
                &self,
                handle: #params_path::ParamHandle,
                normalized: f64,
            ) -> ::std::result::Result<(), #params_path::ParamError> {
                match handle.index() {
                    #(
                        #param_indices => {
                            self.#param_idents.set_normalized(normalized);
                            ::std::result::Result::Ok(())
                        }
                    )*
                    _ => ::std::result::Result::Err(#params_path::ParamError::Unknown(
                        ::std::format!("handle:{}", handle.index())
                    )),
                }
            }
        }
    })
}

struct ParamField {
    ident: Ident,
    id: Option<LitStr>,
    bypass: bool,
}

impl ParamField {
    fn id_expr(&self) -> TokenStream2 {
        let ident = &self.ident;
        match &self.id {
            Some(id) => quote!(#id),
            None => quote!(self.#ident.id()),
        }
    }

    fn spec_expr(&self) -> TokenStream2 {
        let ident = &self.ident;
        let set_id = self.id.as_ref().map(|id| {
            quote! {
                spec.id = ::std::string::String::from(#id);
            }
        });
        let set_bypass = self.bypass.then(|| {
            quote! {
                spec.flags.bypass = true;
            }
        });

        if set_id.is_none() && set_bypass.is_none() {
            return quote!(self.#ident.spec());
        }

        quote! {{
            let mut spec = self.#ident.spec();
            #set_id
            #set_bypass
            spec
        }}
    }
}

#[derive(Default)]
struct ParamAttrs {
    skip: bool,
    id: Option<LitStr>,
    bypass: bool,
}

fn parse_param_attrs(attrs: &[syn::Attribute]) -> syn::Result<ParamAttrs> {
    let mut parsed = ParamAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("param") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                parsed.skip = true;
                Ok(())
            } else if meta.path.is_ident("id") {
                if parsed.id.is_some() {
                    return Err(meta.error("duplicate #[param(id = ...)] attribute"));
                }
                let value = meta.value()?;
                parsed.id = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("bypass") {
                parsed.bypass = true;
                Ok(())
            } else {
                Err(meta.error(
                    "unsupported #[param] attribute; expected skip, id = \"...\", or bypass",
                ))
            }
        })?;
    }
    if parsed.skip && (parsed.id.is_some() || parsed.bypass) {
        return Err(syn::Error::new(
            parsed
                .id
                .as_ref()
                .map_or_else(proc_macro2::Span::call_site, Spanned::span),
            "#[param(skip)] cannot be combined with id or bypass",
        ));
    }
    Ok(parsed)
}

fn is_supported_param_type(ty: &Type) -> bool {
    is_named_param_type(ty, &["FloatParam", "BoolParam", "ChoiceParam"])
}

fn is_bool_param_type(ty: &Type) -> bool {
    is_named_param_type(ty, &["BoolParam"])
}

fn is_named_param_type(ty: &Type, names: &[&str]) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| names.contains(&segment.ident.to_string().as_str()))
}

fn params_module_path() -> TokenStream2 {
    if let Some(vesty) = crate_path("vesty") {
        return quote!(#vesty::params);
    }
    if let Some(params) = crate_path("vesty-params") {
        return quote!(#params);
    }
    quote!(::vesty::params)
}

fn crate_path(package: &str) -> Option<TokenStream2> {
    match crate_name(package).ok()? {
        FoundCrate::Itself => Some(quote!(crate)),
        FoundCrate::Name(name) => {
            let ident = format_ident!("{}", name);
            Some(quote!(::#ident))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn derive_accepts_id_and_bypass_attributes() {
        let input: DeriveInput = parse_quote! {
            struct Params {
                #[param(id = "wet")]
                mix: FloatParam,
                #[param(bypass)]
                bypass: BoolParam,
            }
        };

        expand_params(input).unwrap();
    }

    #[test]
    fn derive_rejects_bypass_on_non_bool_param() {
        let input: DeriveInput = parse_quote! {
            struct Params {
                #[param(bypass)]
                gain: FloatParam,
            }
        };

        let error = expand_params(input).unwrap_err();

        assert!(error.to_string().contains("BoolParam"));
    }

    #[test]
    fn derive_rejects_skip_combined_with_other_param_attrs() {
        let input: DeriveInput = parse_quote! {
            struct Params {
                #[param(skip, id = "ignored")]
                label: String,
            }
        };

        let error = expand_params(input).unwrap_err();

        assert!(error.to_string().contains("cannot be combined"));
    }

    #[test]
    fn derive_rejects_duplicate_id_attribute() {
        let input: DeriveInput = parse_quote! {
            struct Params {
                #[param(id = "a", id = "b")]
                gain: FloatParam,
            }
        };

        let error = expand_params(input).unwrap_err();

        assert!(error.to_string().contains("duplicate"));
    }

    #[test]
    fn vst3_panic_boundary_wraps_callbacks_with_typed_fallbacks() {
        let input: ItemImpl = parse_quote! {
            impl Trait for Type {
                unsafe fn call(&self) -> tresult {
                    panic!("boom")
                }

                unsafe fn pointer(&self) -> *mut u8 {
                    panic!("boom")
                }
            }
        };

        let expanded = expand_vst3_panic_boundary(input).to_string();

        assert!(expanded.contains("catch_unwind"));
        assert!(expanded.contains("kResultFalse"));
        assert!(expanded.contains("Default :: default"));
    }
}
