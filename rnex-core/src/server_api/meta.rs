use rnex_server_api::meta::{
    ApiFeature, ApiFeatures, BuildInfo, server_meta_service_server::ServerMetaService,
};
use tonic::{Request, Response, Status, async_trait, transport::Server};

pub struct ServerMeta;

#[async_trait]
impl ServerMetaService for ServerMeta {
    async fn get_build_info(&self, request: Request<()>) -> Result<Response<BuildInfo>, Status> {
        Ok(Response::new(BuildInfo {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            edition: env!("EDITION").to_owned(),
            build_hash: env!("GIT_HASH").to_owned(),
            feature_set: env!("FEATURESET").to_owned(),
        }))
    }
    async fn get_api_features(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiFeatures>, Status> {
        let mut api_features = vec![ApiFeature {
            name: "meta".to_owned(),
            needs_admin: false,
            version: 0,
        }];

        #[cfg(not(feature = "friends"))]
        api_features.push(ApiFeature {
            name: "gatherings".to_owned(),
            version: 0,
            needs_admin: true,
        });
        Ok(Response::new(ApiFeatures { api_features }))
    }
}
