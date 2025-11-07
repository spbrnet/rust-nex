//! Legacy grpc communication server for being able to use this with pretendos infrastructure
//! before account rs is finished.
//! 
//! This WILL be deprecated as soon as account rs is in a stable state.
use tonic::{Request, Status};

type InterceptorFunc = Box<dyn Fn(Request<()>) -> Result<Request<()>, Status> + Send>;
pub mod account;