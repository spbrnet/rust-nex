use std::net::SocketAddr;

use rnex_auth::{is_maintenance, set_maintenance};
use rnex_server_api::auth::{
    auth_admin_service_server::{AuthAdminService, AuthAdminServiceServer},
    MaintenanceState, Empty,
};
use tonic::{Request, Response, Status, transport::Server};

pub struct AdminGrpc {
    token: Option<String>,
}

impl AdminGrpc {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    fn authorize<T>(&self, request: &Request<T>) -> Result<(), Status> {
        let Some(expected) = &self.token else {
            return Ok(());
        };

        let Some(value) = request.metadata().get("authorization") else {
            return Err(Status::unauthenticated("missing authorization header"));
        };

        let header = value
            .to_str()
            .map_err(|_| Status::unauthenticated("authorization header must be ASCII"))?;

        let supplied = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or_else(|| Status::unauthenticated("authorization header must use Bearer scheme"))?;

        if supplied == expected {
            Ok(())
        } else {
            Err(Status::unauthenticated("invalid admin token"))
        }
    }
}

#[tonic::async_trait]
impl AuthAdminService for AdminGrpc {
    async fn set_maintenance(
        &self,
        request: Request<MaintenanceState>,
    ) -> Result<Response<Empty>, Status> {
        self.authorize(&request)?;
        set_maintenance(request.into_inner().enabled);
        Ok(Response::new(Empty {}))
    }

    async fn get_maintenance(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<MaintenanceState>, Status> {
        self.authorize(&request)?;
        Ok(Response::new(MaintenanceState {
            enabled: is_maintenance(),
        }))
    }
}

pub async fn serve(addr: SocketAddr, token: Option<String>) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(AuthAdminServiceServer::new(AdminGrpc::new(token)))
        .serve(addr)
        .await
}