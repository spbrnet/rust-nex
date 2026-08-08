use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    DataEnum, DataStruct, DeriveInput, Field, Fields, Ident, LitStr, Meta, Token, Variant,
    bracketed, parse::Parse, punctuated::Punctuated, token::Bracket,
};

use crate::util::fold_tokenable;

struct RmcStructAttrVersion {
    bracket: Bracket,
    delim: Token![,],
    feature_name: Literal,
    struct_version: Literal,
}

struct RmcStructAttr {
    base_ver: Literal,
    versions: Option<(Token![,], Punctuated<RmcStructAttrVersion, Token![,]>)>,
}

impl Parse for RmcStructAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let base_ver = input.parse()?;

        if let Some(seperator) = input.parse()? {
            let mut punctuated = Punctuated::new();
            loop {
                punctuated.push_value(input.parse()?);
                if let Some(punct) = input.parse()? {
                    punctuated.push_punct(punct);
                } else {
                    return Ok(Self {
                        base_ver,
                        versions: Some((seperator, punctuated)),
                    });
                }
            }
        } else {
            Ok(Self {
                base_ver,
                versions: None,
            })
        }
    }
}

impl RmcStructAttr {
    fn versions(&self) -> impl Iterator<Item = &RmcStructAttrVersion> {
        self.versions.iter().flat_map(|v| v.1.iter())
    }
}

impl Parse for RmcStructAttrVersion {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let bracket = bracketed!(content in input);
        let (feature_name, delim, struct_version) =
            content.call(|s| Ok((s.parse()?, s.parse()?, s.parse()?)))?;

        Ok(Self {
            bracket,
            delim,
            feature_name,
            struct_version,
        })
    }
}

pub fn generate_write_size_struct(
    s: &DataStruct,
    with_potential_header: bool,
) -> proc_macro2::TokenStream {
    // this is fine and works because of a quirk where the sizes of the structs dont change
    //  if we ignore wether or not a struct extends the other struct or has it as a field

    let base_size = fold_tokenable(s.fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let attrs = fold_tokenable(f.attrs.iter().filter(|a| {
            if let Some(i) = a.meta.path().get_ident() {
                i != "extends"
            } else {
                true
            }
        }));
        quote! {
            #attrs
            let sum = sum + ::rnex_rmc::serialization::RmcSerialize::serialize_write_size(&self.#ident)?;
        }
    }));
    let optional_struct_header_calc = if with_potential_header {
        quote! { let sum = sum + (if ::rnex_rmc::config::FEATURE_HAS_STRUCT_HEADER{ 5 } else { 0 }); }
    } else {
        quote! {}
    };
    quote! {
        let mut sum = 0;
        #base_size
        #optional_struct_header_calc
        Ok(sum)
    }
}
pub fn generate_serialize_struct(
    extended_struct: Option<&Field>,
    elems: &[&Field],
    with_header: bool,
) -> proc_macro2::TokenStream {
    fn gen_elem_serialize(f: &Field) -> TokenStream {
        let ident = f.ident.as_ref().unwrap();
        let attrs = fold_tokenable(f.attrs.iter().filter(|a| {
            if let Some(i) = a.meta.path().get_ident() {
                i != "extends"
            } else {
                true
            }
        }));
        quote! {
            #attrs
            ::rnex_rmc::serialization::RmcSerialize::serialize(&self.#ident, writer)?;
        }
    }
    let optional_extended_struct = if let Some(f) = extended_struct {
        gen_elem_serialize(f)
    } else {
        quote! {}
    };
    let elems = fold_tokenable(elems.iter().map(|e| gen_elem_serialize(e)));
    let ser_body = if with_header {
        quote! {
            ::rnex_rmc::rmc_struct::write_struct(
                    writer,
                    Self::version().unwrap(),
                    ::rnex_rmc::helpers::len_of_write(
                        |writer|{
                            #elems
                            Ok(())
                        }
                    ),
                    |writer|{
                        #elems
                        Ok(())
                    }
                )?;
        }
    } else {
        elems
    };

    quote! {
        #optional_extended_struct
        #ser_body
        Ok(())
    }
}
pub fn generate_deserialize_struct(
    s: &DataStruct,
    extended_struct: Option<&Field>,
    elems: &[&Field],
    with_header: bool,
) -> proc_macro2::TokenStream {
    fn gen_elem_serialize(f: &Field) -> TokenStream {
        let ident = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let attrs = fold_tokenable(f.attrs.iter().filter(|a| {
            if let Some(i) = a.meta.path().get_ident() {
                i != "extends"
            } else {
                true
            }
        }));
        quote! {
            #attrs
            let #ident: #ty = ::rnex_rmc::serialization::RmcSerialize::deserialize(reader)?;
        }
    }
    let optional_extended_struct = if let Some(f) = extended_struct {
        gen_elem_serialize(f)
    } else {
        quote! {}
    };
    let elems = fold_tokenable(elems.iter().map(|e| gen_elem_serialize(e)));
    let struct_ctor_content = fold_tokenable(s.fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let attrs = fold_tokenable(f.attrs.iter().filter(|a| {
            if let Some(i) = a.meta.path().get_ident() {
                i != "extends"
            } else {
                true
            }
        }));
        quote! { #attrs #ident, }
    }));
    let de_body_inner = quote! {
        #elems
        Ok(Self{
            #struct_ctor_content
        })
    };
    let de_body = if with_header {
        quote! {
            ::rnex_rmc::rmc_struct::read_struct(reader, Self::version().unwrap(), move |mut reader|{
                #de_body_inner
            })
        }
    } else {
        de_body_inner
    };

    quote! {
        #optional_extended_struct
        #de_body
    }
}

fn generate_struct_version(attr: Option<&RmcStructAttr>) -> proc_macro2::TokenStream {
    if let Some(attr) = attr {
        let base_ver = &attr.base_ver;
        let if_else_chain = fold_tokenable(attr.versions().map(|v| {
            let version_val = &v.struct_version;
            let feature = &v.feature_name;
            quote! {
                if cfg!(feature = #feature){
                    #version_val
                } else
            }
        }));

        quote! {
            Some(#if_else_chain {
                #base_ver
            })
        }
    } else {
        quote! { None }
    }
}

fn gen_rmc_struct_impl(
    struct_ident: &Ident,
    extended_struct: Option<&Field>,
) -> proc_macro2::TokenStream {
    let self_name_str_lit = LitStr::new(&struct_ident.to_string(), struct_ident.span());
    let register = if let Some(extended_struct) = extended_struct {
        let extended_struct_ty = &extended_struct.ty;
        let ext_ty_name = LitStr::new(
            &extended_struct_ty.to_token_stream().to_string(),
            Span::call_site(),
        );
        quote! {
            #[::ctor::ctor(unsafe)]
            #[allow(nonstandard_style)]
            fn register_fun() {
                println!("registering {} as parent of {}", #self_name_str_lit, #ext_ty_name);
                let mut wr = <#extended_struct_ty as ::rnex_rmc::serialization::RmcStruct>::get_struct_info()
                    .inheritors
                    .write()
                    .expect("poisoned");
                wr.push(<#struct_ident as ::rnex_rmc::serialization::RmcStruct>::get_struct_info());
            }
        }
    } else {
        quote! {}
    };

    quote! {
        impl ::rnex_rmc::serialization::RmcStruct for #struct_ident {
            fn get_struct_info() -> &'static ::rnex_rmc::serialization::RmcStructInfo {
                #register
                static STRUCT_DATA: ::rnex_rmc::serialization::RmcStructInfo =
                    ::rnex_rmc::serialization::RmcStructInfo {
                        inheritors: ::std::sync::RwLock::new(::std::vec::Vec::new()),
                        name: #self_name_str_lit,
                    };

                &STRUCT_DATA
            }
        }

    }
}

pub fn rmc_serialize_struct(
    s: &DataStruct,
    name: &Ident,
    derive_input: &DeriveInput,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
) {
    let struct_attr = derive_input.attrs.iter().find(|a| {
        a.path().segments.len() == 1
            && a.path()
                .segments
                .first()
                .is_some_and(|p| p.ident == "rmc_struct")
            && matches!(a.meta, Meta::List(_))
    });

    let struct_attr: Option<RmcStructAttr> = struct_attr.map(|a| a.parse_args().unwrap());
    let struct_attr = struct_attr.as_ref();

    let extended_struct = s.fields.iter().find(|f| {
        f.attrs.iter().any(|a| {
            a.path().segments.len() == 1
                && a.path()
                    .segments
                    .first()
                    .is_some_and(|p| p.ident == "extends")
        })
    });
    let elements: Vec<_> = s
        .fields
        .iter()
        .filter(|f| {
            !f.attrs.iter().any(|a| {
                a.path().segments.len() == 1
                    && a.path()
                        .segments
                        .first()
                        .is_some_and(|p| p.ident == "extends")
            })
        })
        .collect();
    let elements = &elements[..];

    let serialize = generate_serialize_struct(extended_struct, elements, struct_attr.is_some());
    let deserialize =
        generate_deserialize_struct(s, extended_struct, elements, struct_attr.is_some());
    let write_size = generate_write_size_struct(s, struct_attr.is_some());
    let version = generate_struct_version(struct_attr);
    let rmc_struct_impl = if struct_attr.is_some() {
        Some(gen_rmc_struct_impl(name, extended_struct))
    } else {
        None
    };

    (
        serialize,
        deserialize,
        Some(write_size),
        Some(version),
        rmc_struct_impl,
    )
}

fn field_to_ident(field: &Field, idx: usize) -> Ident {
    if let Some(i) = &field.ident {
        i.clone()
    } else {
        Ident::new(&format!("field_{}", idx), Span::call_site())
    }
}

fn variant_to_pattern_and_fields(variant: &Variant) -> (proc_macro2::TokenStream, Vec<Field>) {
    match &variant.fields {
        Fields::Named(n) => {
            let inner = n
                .named
                .iter()
                .map(|f| {
                    let attrs = fold_tokenable(f.attrs.iter());
                    let ident = f.ident.as_ref().unwrap();

                    quote! { #attrs #ident }
                })
                .reduce(|a, b| quote! {#a, #b});

            (quote! {{#inner}}, n.named.iter().cloned().collect())
        }
        Fields::Unnamed(n) => {
            let inner = n
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let attrs = fold_tokenable(f.attrs.iter());
                    let name = field_to_ident(f, i);

                    quote! { #attrs #name }
                })
                .reduce(|a, b| quote! {#a, #b});

            (quote! {(#inner)}, n.unnamed.iter().cloned().collect())
        }
        Fields::Unit => (quote! {}, vec![]),
    }
}

pub fn rmc_generate_serialize_enum(
    enum_data: &DataEnum,
    repr_ty: &Ident,
) -> proc_macro2::TokenStream {
    let match_content = fold_tokenable(enum_data.variants.iter().map(|v| {
        let ident = &v.ident;
        let descriminant = &v
            .discriminant
            .as_ref()
            .expect("every variant must have a descriminant to be a valid rmc struct")
            .1;
        let (pattern, fields) = variant_to_pattern_and_fields(v);
        let inner = fold_tokenable(fields.iter().enumerate().map(|(i, f)| {
            let ty = &f.ty;
            let name = field_to_ident(f, i);
            quote! {<#ty as ::rnex_rmc::serialization::RmcSerialize>::serialize(#name, writer)?;}
        }));
        quote! {
            Self::#ident #pattern => {
                <#repr_ty as ::rnex_rmc::serialization::RmcSerialize>::serialize(&#descriminant, writer)?;
                #inner
            }
        }
    }));
    quote! {
        match self{
            #match_content
        }
        Ok(())
    }
}
pub fn rmc_generate_deserialize_enum(
    enum_data: &DataEnum,
    repr_ty: &Ident,
) -> proc_macro2::TokenStream {
    let match_content = fold_tokenable(enum_data.variants.iter().map(|v| {
        let ident = &v.ident;
        let descriminant = &v
            .discriminant
            .as_ref()
            .expect("every variant must have a descriminant to be a valid rmc struct")
            .1;
        let (pattern, fields) = variant_to_pattern_and_fields(v);
        let inner = fold_tokenable(fields.iter().enumerate().map(|(i, f)| {
            let ty = &f.ty;
            let name = field_to_ident(f, i);
            quote! {let #name = <#ty as ::rnex_rmc::serialization::RmcSerialize>::deserialize(reader)?;}
        }));
        quote! {
            #descriminant => {
                #inner

                Self::#ident #pattern
            }
        }
    }));

    quote! {
        let discriminant = <#repr_ty as ::rnex_rmc::serialization::RmcSerialize>::deserialize(reader)?;

        Ok(match discriminant{
            #match_content
            v => {
                return Err(::rnex_rmc::serialization::Error::UnexpectedValue(v as u64))
            }
        })
    }
}

pub fn rmc_serialize_enum(
    enum_data: &DataEnum,
    derive_input: &DeriveInput,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
) {
    let repr_attr = derive_input.attrs.iter().find(|a| {
        a.path().segments.len() == 1 && a.path().segments.first().is_some_and(|p| p.ident == "repr")
    });
    let Some(repr_attr) = repr_attr else {
        panic!("missing repr attribute");
    };

    let ty: Ident = repr_attr.parse_args().unwrap();

    let serialize = rmc_generate_serialize_enum(enum_data, &ty);
    let deserialize = rmc_generate_deserialize_enum(enum_data, &ty);

    (serialize, deserialize, None, None, None)
}
