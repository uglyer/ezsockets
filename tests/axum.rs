mod chat;
mod client;

use chat::ChatClient;
use chat::ChatServer;

use axum_crate as axum;

use axum::extract::Extension;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use ezsockets::axum::Upgrade;
use ezsockets::Server;
use ezsockets::ServerExt;
use ezsockets::SessionExt;
use std::net::SocketAddr;

async fn websocket_handler<E>(
    Extension(server): Extension<Server<E>>,
    ezsocket: Upgrade,
) -> impl IntoResponse
where
    E: ServerExt + 'static,
    <E::Session as SessionExt>::Args: Default,
{
    ezsocket.on_upgrade(server, Default::default())
}

async fn run<E>(create_fn: impl FnOnce(Server<E>) -> E) -> (Server<E>, SocketAddr)
where
    E: ServerExt + 'static,
    <E::Session as SessionExt>::Args: Default,
{
    let (server, _) = Server::create(create_fn);
    let app = Router::new()
        .route("/websocket", get(websocket_handler::<E>))
        .layer(Extension(server.clone()));

    let address = SocketAddr::from(([127, 0, 0, 1], 0));

    tracing::debug!("listening on {}", address);
    let future =
        axum::Server::bind(&address).serve(app.into_make_service_with_connect_info::<SocketAddr>());
    let address = future.local_addr();
    tokio::spawn(async move {
        future.await.unwrap();
    });
    (server, address)
}

#[tokio::test]
async fn test_axum_chat() {
    tracing_subscriber::fmt::init();
    let (_, address) = run(ChatServer::new).await;
    let alice = client::connect(ChatClient::new, address).await;
    let bob = client::connect(ChatClient::new, address).await;
    chat::test(alice, bob).await;
}
