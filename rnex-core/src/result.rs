use std::error::Error;
use log::error;

pub trait ResultExtension{
    type Output;

    fn display_err_or_some(self) -> Option<Self::Output>;
}

impl<T, U: Error> ResultExtension for Result<T, U>{
    type Output = T;

    fn display_err_or_some(self) -> Option<Self::Output> {
        match self{
            Ok(v) => Some(v),
            Err(e) => {
                error!("{}", e);

                None
            }
        }
    }
}