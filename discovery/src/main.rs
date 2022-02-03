use crate::model::Model;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{AddExtensionLayer, Json, Router};
use log::error;
use shared::discovery::*;
use shared::initialize_logger;

use std::sync::Arc;
use tokio::runtime::Runtime;

mod model;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BoxResult<T> = Result<T, BoxError>;

struct State {
    model: Arc<Model>,
}

async fn register(
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let instance_id = state.model.register_instance(&payload.address).await;

    match instance_id {
        Ok(instance_id) => Ok(Json::from(RegisterResponse { instance_id })),
        Err(error) => {
            error!(
                "An error occurred while registering a new instance - {:?}",
                error
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn ping(
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<PingRequest>,
) -> Result<Json<PingResponse>, StatusCode> {
    let result = state
        .model
        .ping_instance(&payload.address, payload.instance_id)
        .await;

    match result {
        Ok(ping_result) => Ok(Json::from(PingResponse { ping_result })),
        Err(error) => {
            error!(
                "An error occurred while receiving a ping from an instance - {:?}",
                error
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn chatroom(
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<ChatroomRequest>,
) -> Result<Json<ChatroomResponse>, StatusCode> {
    let result = state.model.get_chatroom(&payload.term).await;

    match result {
        Ok(instance) => Ok(Json::from(ChatroomResponse { instance })),
        Err(error) => {
            error!(
                "An error occurred while receiving fetching the address of a chatroom - {:?}",
                error
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn async_main() -> BoxResult<()> {
    let model = Arc::new(Model::new().await?);
    let state = Arc::new(State { model });

    let app = Router::new()
        .route("/register", post(register))
        .route("/ping", post(ping))
        .route("/chatroom", post(chatroom))
        .layer(AddExtensionLayer::new(state));

    axum::Server::bind(&"0.0.0.0:8081".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn main() -> BoxResult<()> {
    initialize_logger()?;
    dotenv::dotenv().ok();

    let runtime = Runtime::new()?;

    runtime.block_on(async_main())?;

    Ok(())
}
