use std::{convert::TryInto, net::SocketAddr, sync::Arc};
use rnex_server::{ConnectionRegistry, util::PID};
use rnex_server_api::meta::{admin_service_server::{AdminService, AdminServiceServer}, KickPidsRequest, KickPidsResponse, OnlinePidsResponse};
use tonic::{Request, Response, Status, transport::Server};
use rnex_server_api::meta::Empty;

pub struct AdminGrpc {
    registry: Arc<ConnectionRegistry>,
    token: Option<String>,
}

impl AdminGrpc {
    pub fn new(registry: Arc<ConnectionRegistry>, token: Option<String>) -> Self {
        Self { registry, token }
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
impl AdminService for AdminGrpc {
    async fn kick_pids(
        &self,
        request: Request<KickPidsRequest>,
    ) -> Result<Response<KickPidsResponse>, Status> {
        self.authorize(&request)?;

        let KickPidsRequest { pids, reason } = request.into_inner();

        rnex_server::tracing::info!(%reason, pid_count = pids.len(), "received kick request");

        let mut kicked = Vec::new();
        let mut missing = Vec::new();

        for raw_pid in pids {
            let pid: PID = raw_pid
                .try_into()
                .map_err(|_| Status::invalid_argument(format!("pid {raw_pid} is not a valid pid for this server")))?;

            if self.registry.kick(pid).await {
                kicked.push(raw_pid);
            } else {
                missing.push(raw_pid);
            }
        }

        Ok(Response::new(KickPidsResponse { kicked, missing }))
    }

    async fn list_online_pids(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<OnlinePidsResponse>, Status> {
        self.authorize(&request)?;

        let pids = self
            .registry
            .online_pids()
            .await
            .into_iter()
            .map(|pid| {
                pid.try_into().map_err(|_| {
                    Status::internal(format!("pid {pid} is not a valid pid for this server"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Response::new(OnlinePidsResponse { pids }))
    }
}

pub async fn serve(
    addr: SocketAddr,
    registry: Arc<ConnectionRegistry>,
    token: Option<String>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(AdminServiceServer::new(AdminGrpc::new(registry, token)))
        .serve(addr)
        .await
}