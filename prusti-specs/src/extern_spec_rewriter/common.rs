use quote::{quote, ToTokens};
use syn::parse_quote_spanned;
use syn::spanned::Spanned;

/// Add `PhantomData` markers for each type parameter to silence errors
/// about unused type parameters.
///
/// Given
/// ```text
/// struct Foo<A,B> {
/// }
/// ```
/// Result
/// ```text
/// struct Foo<A,B> {
///     ::core::marker::PhantomData<A>,
///     ::core::marker::PhantomData<B>
/// }
/// ```
pub fn add_phantom_data_for_generic_params(item_struct: &mut syn::ItemStruct) {
    let fields = item_struct.generics.params.iter()
        .flat_map(|param| match param {
            syn::GenericParam::Type(tp) => {
                let ident = tp.ident.clone();
                Some(quote!(::core::marker::PhantomData<#ident>))
            }
            syn::GenericParam::Lifetime(ld) => {
                let ident = ld.lifetime.clone();
                Some(quote!(&#ident ::core::marker::PhantomData<()>))
            }
            syn::GenericParam::Const(_cp) => None,
        });

    item_struct.fields = syn::Fields::Unnamed(syn::parse_quote! { ( #(#fields),* ) });
}

/// We take the Generics (parameters) defined with the `#[extern_spec] impl<...>` (the `<...>`)
/// but then need to pass those as arguments: `SomeStruct<...>`. This function translates from
/// the syntax of one to the other; e.g. `<T: Bound, 'l: Bound, const C: usize>` -> `<T, 'l, C>`
pub fn rewrite_generics(gens: &syn::Generics) -> syn::AngleBracketedGenericArguments {
    let args: Vec<syn::GenericArgument> = gens
        .params
        .clone()
        .into_iter()
        .map(|gp| {
            let ts = match gp {
                syn::GenericParam::Type(syn::TypeParam { ident, .. })
                | syn::GenericParam::Const(syn::ConstParam { ident, .. }) => ident.into_token_stream(),
                syn::GenericParam::Lifetime(ld) => ld.lifetime.into_token_stream(),
            };
            syn::parse2::<syn::GenericArgument>(ts).unwrap()
        })
        .collect();
    syn::parse_quote! { < #(#args),* > }
}

/// Given a replacement for the `Self` type and the trait it should fulfill,
/// this type rewrites `Self` and associated type paths.
///
/// # Example
/// Given a `Self` replacement `T_Self` and a self trait constraint `Foo<X>`,
/// visiting a function
/// ```
/// fn foo(arg1: Self, arg2: Self::Assoc1) -> Self::Assoc2 { }
/// ```
/// results in
/// ```
/// fn foo(arg1: T_Self, arg2: <T_Self as Foo<X>::Assoc1) -> <T_Self as Foo<X>::Assoc2 { }
/// ```
pub struct AssociatedTypeRewriter<'a> {
    self_type: &'a syn::TypePath,
    self_type_trait: &'a syn::TypePath,
}

impl<'a> AssociatedTypeRewriter<'a> {
    pub fn new(
        self_type: &'a syn::TypePath,
        self_type_trait: &'a syn::TypePath,
    ) -> Self {
        AssociatedTypeRewriter {
            self_type,
            self_type_trait,
        }
    }

    pub fn rewrite_method_sig(&mut self, signature: &mut syn::Signature) {
        syn::visit_mut::visit_signature_mut(self, signature);
    }
}

impl<'a> syn::visit_mut::VisitMut for AssociatedTypeRewriter<'a> {
    fn visit_type_path_mut(&mut self, ty_path: &mut syn::TypePath) {
        if ty_path.qself.is_none()
            && !ty_path.path.segments.is_empty()
            && ty_path.path.segments[0].ident == "Self" {
            if ty_path.path.segments.len() == 1 {
                // replace `Self` type
                *ty_path = self.self_type.clone();
            } else if ty_path.path.segments.len() >= 2 {
                // replace associated types
                let mut path_rest = ty_path.path.segments.clone()
                    .into_pairs()
                    .skip(1)
                    .collect::<syn::punctuated::Punctuated::<syn::PathSegment, _>>();
                if ty_path.path.segments.trailing_punct() {
                    path_rest.push_punct(<syn::Token![::]>::default());
                }
                let self_type = &self.self_type;
                let self_type_trait = &self.self_type_trait;
                *ty_path = parse_quote_spanned! {ty_path.span()=>
                    < #self_type as #self_type_trait > :: #path_rest
                };
            }
        }
        syn::visit_mut::visit_type_path_mut(self, ty_path);
    }
}

