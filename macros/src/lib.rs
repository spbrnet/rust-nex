#![allow(dead_code)]
mod protos;
mod rmc_struct;
mod util;

extern crate proc_macro;

use crate::protos::{ProtoInputParams, RmcProtocolData};
use crate::rmc_struct::{rmc_serialize_enum, rmc_serialize_struct};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(RmcSerialize, attributes(extends, rmc_struct))]
pub fn rmc_serialize(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let (serialize, deserialize, write_size, version, rmc_struct_impl) = match &derive_input.data {
        Data::Struct(s) => rmc_serialize_struct(s, &derive_input.ident, &derive_input),
        Data::Enum(e) => rmc_serialize_enum(e, &derive_input),
        Data::Union(_) => {
            unimplemented!("serializing a union is not allowed");
        }
    };

    // generate base data

    let ident = derive_input.ident;

    let write_size = if let Some(v) = write_size {
        quote! {
            fn serialize_write_size(&self) -> rnex_core::rmc::structures::Result<u32>{
                #v
            }
        }
    } else {
        quote! {}
    };

    let version = if let Some(v) = version {
        quote! {
            fn version() -> Option<u8>{
                #v
            }
        }
    } else {
        quote! {}
    };
    let rmc_struct_impl = rmc_struct_impl.unwrap_or_default();

    let tokens = quote! {
        impl rnex_core::rmc::structures::RmcSerialize for #ident{
            #[inline(always)]
            fn serialize(&self, writer: &mut (impl ::std::io::Write + ?::std::marker::Sized)) -> rnex_core::rmc::structures::Result<()>{
                #serialize
            }
            #[inline(always)]
            fn deserialize(reader: &mut (impl ::std::io::Read + ?::std::marker::Sized)) -> rnex_core::rmc::structures::Result<Self>{
                #deserialize
            }

            #write_size

            #version
        }
        #rmc_struct_impl
    };

    tokens.into()
}

/// Macro to automatically generate code to use a specific trait as an rmc protocol for calling to
/// remote objects or accepting incoming remote requests.
/// This is needed in order to be able to use this as part of an rmc server interface.
///
/// The protocol id which is needed to be specified is specified as a parameter to this attribute.
///
/// You will also need to assign each function inside the trait a method id by using the
/// [`macro@method_id`] attribute.
///
/// You can also specify to have the protocol to be non-returning by adding a second parameter to
/// the attribute which is just `NoReturn` e.g. `#[rmc_proto(1, NoReturn)]`
///
/// Example
/// ```
/// // this rmc protocol has protocol id 1
/// use macros::rmc_proto;
///
/// #[rmc_proto(1)]
/// trait ExampleProtocol{
///     // this defines an rmc method with id 1
///     #[rmc_method(1)]
///     async fn hello_world_method(&self, name: String) -> Result<String, ErrorCode>;
/// }
/// ```
#[proc_macro_attribute]
pub fn rmc_proto(attr: TokenStream, input: TokenStream) -> TokenStream {
    let params = parse_macro_input!(attr as ProtoInputParams);
    let input = parse_macro_input!(input as syn::ItemTrait);

    let raw_data = RmcProtocolData::new(params, &input);

    quote! {
        #input
        #raw_data
    }
    .into()
}

/// Used to specify the method id of methods when making rmc protocols.
/// See [`macro@rmc_proto`] for further details.
///
/// Note: This attribute doesn't do anything by itself and just returns the thing it was attached to
/// unchanged.
#[proc_macro_attribute]
pub fn method_id(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // this attribute doesnt do anything by itself, see `rmc_proto`
    input
}

#[proc_macro_attribute]
pub fn rmc_struct(attr: TokenStream, input: TokenStream) -> TokenStream {
    let type_data = parse_macro_input!(input as DeriveInput);
    let mut ident = parse_macro_input!(attr as syn::Path);
    let last_token = ident.segments.last_mut().expect("empty path?");

    last_token.ident = Ident::new(
        &("Local".to_owned() + &last_token.ident.to_string()),
        last_token.span(),
    );

    let struct_name = &type_data.ident;

    let out = quote! {
        #type_data

        impl #ident for #struct_name{

        }

        impl rnex_core::rmc::protocols::RmcCallable for #struct_name{
            async fn rmc_call(&self, remote_response_connection: &rnex_core::util::SendingBufferConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: Vec<u8>){
                <Self as #ident>::rmc_call(self, remote_response_connection, protocol_id, method_id, call_id, rest).await;
            }
        }
    };

    out.into()
}

#[proc_macro_attribute]
pub fn connection(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // this attribute doesnt do anything by itself, see `rmc_struct`
    input
}
