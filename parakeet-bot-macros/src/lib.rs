use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;
use syn::spanned::Spanned;
use syn::Error;
use syn::{
    Attribute, Block, Ident, ImplItem, ImplItemMethod, ItemFn, ItemImpl, Result, Signature,
    Visibility,
};

/// Represents a set of arguments given to a proc macro.
///
/// Parses a list of identifiers seperated by a comma.
/// Example:
///     [#my_macrovar(1, fn_a, my_struct)]
///                   ^^^^^^^^^^^^^^^^^^
///
#[derive(Debug)]
struct Args {
    wrapper_fun: Ident,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let wrapper_fun = Ident::parse(input)?;
        Ok(Args { wrapper_fun })
    }
}

#[derive(Debug)]
enum MyFn {
    ItemFn(ItemFn),
    Impl(ItemImpl),
    ImplItemMethod(ImplItemMethod),
}

impl Parse for MyFn {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Ok(itemfn) = ItemFn::parse(input) {
            let item = Self::ItemFn(itemfn);
            item.assert_ret_type()?;
            Ok(item)
        } else if let Ok(implfn) = ImplItemMethod::parse(input) {
            let item = Self::ImplItemMethod(implfn);
            item.assert_ret_type()?;
            Ok(item)
        } else if let Ok(item_impl) = ItemImpl::parse(input) {
            let item = Self::Impl(item_impl);
            item.assert_ret_type()?;
            Ok(item)
        } else {
            Err(Error::new(input.span(), "expected function or impl"))
        }
    }
}

impl ToTokens for MyFn {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            MyFn::ItemFn(itemfn) => itemfn.to_tokens(tokens),
            MyFn::ImplItemMethod(implfn) => implfn.to_tokens(tokens),
            MyFn::Impl(im) => im.to_tokens(tokens),
        }
    }
}

impl MyFn {
    fn assert_ret_type(&self) -> Result<()> {
        fn is_sig_ok(sig: &Signature) -> Result<()> {
            let ret_type = &sig.output;
            let has_res = if let syn::ReturnType::Type(_, ty) = ret_type {
                match ty.as_ref() {
                    syn::Type::Path(syn::TypePath { qself: _, path }) => {
                        path.segments.first().unwrap().ident
                            == Ident::new("Result", ret_type.span())
                    }
                    _ => false,
                }
            } else {
                false
            };
            if has_res {
                Ok(())
            } else {
                Err(Error::new(sig.span(), "expected result type"))
            }
        }

        match self {
            MyFn::ItemFn(_) | MyFn::ImplItemMethod(_) => is_sig_ok(self.get_sig().unwrap()),
            MyFn::Impl(imp) => {
                for i in &imp.items {
                    match i {
                        ImplItem::Method(meth) => is_sig_ok(&meth.sig)?,
                        _ => (),
                    };
                }
                Ok(())
            }
        }
    }

    fn get_vis(&self) -> Option<&Visibility> {
        match self {
            MyFn::ItemFn(itemfn) => Some(&itemfn.vis),
            MyFn::ImplItemMethod(implfn) => Some(&implfn.vis),
            _ => None,
        }
    }

    fn get_sig(&self) -> Option<&Signature> {
        match self {
            MyFn::ItemFn(itemfn) => Some(&itemfn.sig),
            MyFn::ImplItemMethod(implfn) => Some(&implfn.sig),
            _ => None,
        }
    }

    fn get_block(&self) -> Option<&Block> {
        match self {
            MyFn::ItemFn(itemfn) => Some(&itemfn.block),
            MyFn::ImplItemMethod(implfn) => Some(&implfn.block),
            _ => None,
        }
    }

    fn wrap(&self, args: &Args) -> Result<TokenStream> {
        let output = move || -> TokenStream {
            let wrapper = &args.wrapper_fun;
            let sig = self.get_sig().unwrap();
            let vis = self.get_vis().unwrap();
            let block = self.get_block().unwrap();
            let has_async = sig.asyncness.is_some();
            let has_self = sig.receiver().is_some();

            // Old function renamed so it can be wrapped around
            let ghost_ident =
                &syn::Ident::new(format!("_ghost_{}", sig.ident).as_ref(), sig.ident.span());
            let ghost_sig = syn::Signature {
                ident: ghost_ident.clone(),
                ..sig.clone()
            };
            let ghost_vis = Visibility::Inherited; // Private function
            let ghost_block = block;

            // Removes return type so it can be originally written there (for aesthetics)
            let new_sig = syn::Signature {
                output: syn::ReturnType::Default,
                ..sig.clone()
            };
            let new_block = match (has_async, has_self) {
                (false, false) => quote! {#wrapper(#ghost_ident())},
                (false, true) => quote! {#wrapper(self.#ghost_ident())},
                (true, false) => quote! {#wrapper(#ghost_ident().await).await},
                (true, true) => quote! {#wrapper(self.#ghost_ident().await).await},
            };

            quote! {
                #ghost_vis #ghost_sig #ghost_block

                #vis #new_sig #new_block
            }
            .into()
        };

        match self {
            MyFn::ItemFn(_) => Ok(output()),
            MyFn::ImplItemMethod(_) => Ok(output()),
            MyFn::Impl(impl_item) => {
                let old_items = &impl_item.items;
                let new_items: Vec<ImplItem> = old_items
                    .iter()
                    .map(|i| match i {
                        ImplItem::Method(meth) => {
                            let a = MyFn::ImplItemMethod(meth.clone()).wrap(args).unwrap();
                            syn::parse2::<ImplItem>(quote! {#a}).unwrap()
                        }
                        _ => i.clone(),
                    })
                    .collect();
                let new_impl_item = ItemImpl {
                    items: new_items,
                    ..impl_item.clone()
                };
                Ok(MyFn::Impl(new_impl_item))
            }
        }
    }
}

#[derive(Debug)]
struct Input {
    attrs: Vec<Attribute>,
    my_fn: MyFn,
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for attr in &self.attrs {
            attr.to_tokens(tokens)
        }
        self.my_fn.to_tokens(tokens)
    }
}

impl Input {
    fn wrap(self, args: &Args) -> Result<TokenStream> {
        let new_fn = self.my_fn.wrap(args)?;
        Ok(new_fn)
    }
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Input {
            attrs: input.call(Attribute::parse_outer)?,
            my_fn: input.parse()?,
        })
    }
}

/// Wraps all `ItemFn`s (function definitions) with the given argument. If the `ItemFn` is `async`,
/// then the wrapper will also be `.await`.
///
/// Example:
/// #[wrap(my_fun)]
/// async fn foo() {...}
///
/// Transforms into:
/// async fn foo() {
///     my_fun(...).await
/// }
#[proc_macro_attribute]
pub fn handle_error(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as Input);
    match input.wrap(&args) {
        Ok(o) => o,
        Err(e) => return e.into_compile_error().into(),
    }
}
// #[proc_macro_attribute]
// pub fn print_ast(args: TokenStream, input: TokenStream) -> TokenStream {
//     // let args = parse_macro_input!(args as Args);
//     let ast = parse_macro_input!(input as syn::Item);
//     // eprintln!("{args:#?}");
//     eprintln!("{ast:#?}");
//     quote!(#ast).into()
// }
