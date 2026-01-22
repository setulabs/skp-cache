use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

#[proc_macro_derive(CacheKey, attributes(cache_key))]
pub fn derive_cache_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse struct attributes
    let mut namespace = None;
    let mut separator = ":".to_string();

    for attr in &input.attrs {
        if attr.path().is_ident("cache_key") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("namespace") {
                    let value = meta.value()?; // this parses the `=`
                    let s: LitStr = value.parse()?; // this parses the string literal "..."
                    namespace = Some(s.value());
                    Ok(())
                } else if meta.path.is_ident("separator") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    separator = s.value();
                    Ok(())
                } else {
                    Err(meta.error("unsupported attribute"))
                }
            });
        }
    }

    // Generate cache_key implementation
    let key_gen = match &input.data {
        Data::Struct(data) => {
            let fields = match &data.fields {
                Fields::Named(fields) => &fields.named,
                Fields::Unnamed(fields) => &fields.unnamed,
                Fields::Unit => return impl_unit_struct(name, namespace),
            };

            let mut key_parts = Vec::new();

            for (i, field) in fields.iter().enumerate() {
                // Check for skip attribute
                let mut skip = false;
                for attr in &field.attrs {
                    if attr.path().is_ident("cache_key") {
                         let _ = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("skip") {
                                skip = true;
                                Ok(())
                            } else {
                                Ok(()) // Ignore other field attributes
                            }
                        });
                    }
                }

                if !skip {
                    if let Some(ident) = &field.ident {
                         key_parts.push(quote! { self.#ident.to_string() });
                    } else {
                        let index = syn::Index::from(i);
                        key_parts.push(quote! { self.#index.to_string() });
                    }
                }
            }

            if key_parts.is_empty() {
                quote! { String::new() }
            } else {
                quote! {
                    let parts = vec![#(#key_parts),*];
                    parts.join(#separator)
                }
            }
        }
        _ => return syn::Error::new_spanned(name, "CacheKey derive only supports structs")
            .to_compile_error()
            .into(),
    };

    let namespace_impl = match namespace {
        Some(ns) => quote! {
            fn namespace(&self) -> Option<&str> {
                Some(#ns)
            }
        },
        None => quote! {},
    };

    let expanded = quote! {
        impl skp_cache_core::CacheKey for #name {
            fn cache_key(&self) -> String {
                #key_gen
            }
            #namespace_impl
        }
    };

    TokenStream::from(expanded)
}

fn impl_unit_struct(name: &syn::Ident, namespace: Option<String>) -> TokenStream {
    let namespace_impl = match namespace {
        Some(ns) => quote! {
            fn namespace(&self) -> Option<&str> {
                Some(#ns)
            }
        },
        None => quote! {},
    };

    let expanded = quote! {
        impl skp_cache_core::CacheKey for #name {
            fn cache_key(&self) -> String {
                String::new()
            }
            #namespace_impl
        }
    };

    TokenStream::from(expanded)
}
