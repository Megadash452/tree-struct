use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl, ImplItemFn, ImplItem};
use quote::{quote, ToTokens};

/// Provide default implementations of functions `iter_bfs`, `iter_dfs`, `debug_tree`, and `ptr` of the **`Node`** trait.
/// 
/// The default implementations can't be included in the trait because these functions require casting `self` to `&dyn Node`,
/// which seems to not be possible because default implementations of trait functions do not know the *concrete type* of `self`,
/// and also don't have the VTable necessary to create the `&dyn Node`.
/// 
/// # Example
/// 
/// The argument of the attribute macro will be used as the *crate name* used by types used in these functions.
/// If imported `tree_struct` under a different name in **`Cargo.toml`**, write the name in the macro parenthesis,
/// or leave it empty to use `tree_struct`.
/// 
/// ```
/// #[full_node_impl(crate)]
/// impl<T> Node for A {
///     fn parent(&self) -> Option<&dyn Node> { ... }
///     fn children(&self) -> Box<[&dyn Node]> { ... }
///     fn debug_content(&self) -> &dyn Debug { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn full_node_impl(name: TokenStream, input: TokenStream) -> TokenStream {
    let name = if name.is_empty() {
        quote! { tree_struct }
    } else {
        proc_macro2::TokenStream::from(name)
    };
    let mut input = parse_macro_input!(input as ItemImpl);

    // Functions of the trait that should have a default impelementation,
    // but don't because of Dynamic Dispatch shenanigans (and why this macro has to exists at all)
    let default_fn = [
        quote! {
            #[inline]
            fn iter_bfs<'a>(&'a self) -> #name::IterBFS<'a> {
                #name::IterBFS::new(self)
            }
        },
        quote! {
            #[inline]
            fn iter_dfs<'a>(&'a self) -> #name::IterDFS<'a> {
                #name::IterDFS::new(self)
            }
        },
        quote! {
            #[inline]
            fn debug_tree(&self) -> #name::DebugTree {
                #name::DebugTree { root: self }
            }
        },
        quote! {
            #[inline]
            fn ptr(&self) -> ::std::ptr::NonNull<dyn Node> {
                ::std::ptr::NonNull::from(self)
            }
        },
    ];

    // The functions of the trait that were implemented in the input impl block.
    let implemented = input.items.iter()
        .filter_map(|item| match item {
            ImplItem::Fn(item_fn) => Some(item_fn.sig.ident.to_string()),
            _ => None
        })
        .collect::<Box<_>>();

    // Functions that will be appended to the output impl block because they were not implemented in the input
    let append = default_fn.into_iter()
        .map(|item_fn| syn::parse2::<ImplItemFn>(item_fn).unwrap())
        .filter_map(|item_fn| (!implemented.contains(&item_fn.sig.ident.to_string())).then_some(item_fn));

    input.items.extend(append.map(|item_fn| ImplItem::Fn(item_fn)));

    println!("{}", input.to_token_stream());

    input.into_token_stream().into()
}
