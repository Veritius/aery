use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, DeriveInput, Error, Ident, Meta,
    Result, Token,
};

struct RelationConfig {
    cleanup: Ident,
    direction: Ident,
    exclusive: bool,
}

fn parse_config(ast: &DeriveInput) -> Result<RelationConfig> {
    let mut cleanup = "Orphan";
    let mut direction = "Directed";
    let mut exclusive = true;

    for attr in ast.attrs.iter().filter(|attr| attr.path().is_ident("aery")) {
        let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in nested {
            match meta {
                Meta::Path(ref path) => {
                    if let Some(new_policy) = ["Counted", "Recursive", "Total"]
                        .into_iter()
                        .find(|ident| path.is_ident(ident))
                    {
                        if cleanup != "Orphan" {
                            return Err(Error::new_spanned(
                                meta,
                                "Tried to set cleanup policy multiple times",
                            ));
                        }
                        cleanup = new_policy;
                    } else if let Some(new_policy) = ["Undirected", "Acyclic"]
                        .into_iter()
                        .find(|ident| path.is_ident(ident))
                    {
                        if direction != "Directed" {
                            return Err(Error::new_spanned(
                                meta,
                                "Tried to set direction policy multiple times",
                            ));
                        }
                        direction = new_policy;
                    } else if path.is_ident("Poly") {
                        if !exclusive {
                            return Err(Error::new_spanned(
                                meta,
                                "Tried to set exclusivity multiple times",
                            ));
                        }
                        exclusive = false;
                    } else {
                        return Err(Error::new_spanned(meta, "Unrecognized property override"));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(meta, "Unrecognized macro format"));
                }
            }
        }
    }

    Ok(RelationConfig {
        cleanup: Ident::new(cleanup, Span::call_site()),
        direction: Ident::new(direction, Span::call_site()),
        exclusive,
    })
}

#[proc_macro_derive(Relation, attributes(aery))]
pub fn relation_derive(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);

    let RelationConfig {
        cleanup,
        direction,
        exclusive,
    } = match parse_config(&ast) {
        Ok(config) => config,
        Err(e) => return e.into_compile_error().into(),
    };

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Sized + Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let output = quote! {
        impl #impl_generics Relation for #struct_name #type_generics #where_clause  {
            const CLEANUP_POLICY: CleanupPolicy = CleanupPolicy::#cleanup;
            const DIRECTION_POLICY: DirectionPolicy = DirectionPolicy::#direction;
            const EXCLUSIVE: bool = #exclusive;
        }
    };

    output.into()
}
