use proc_macro2::TokenStream;
use quote::ToTokens;

// todo: return a wrapper struct implementing ToTokens over the iterator instead as to avoid unnescesary allocations with the token stream
pub fn fold_tokenable<T: ToTokens>(list: impl Iterator<Item = T>) -> TokenStream {
    list.fold(TokenStream::new(), |mut s, i| {
        i.to_tokens(&mut s);
        s
    })
}
