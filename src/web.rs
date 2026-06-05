use embassy_net::Stack;
use embassy_time::Duration;
use esp_alloc as _;
use picoserve::{AppBuilder, AppRouter, Router, response::File, routing};

pub const WEB_TASK_POOL_SIZE: usize = 2;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(
    task_id: usize,
    stack: Stack<'static>,
    router: &'static AppRouter<Application>,
    config: &'static picoserve::Config,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::Server::new(router, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}

pub fn web_app() -> (
    &'static Router<<Application as AppBuilder>::PathRouter>,
    &'static picoserve::Config,
) {
    let router = picoserve::make_static!(AppRouter<Application>, Application.build_app());

    let config = picoserve::make_static!(
        picoserve::Config,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Duration::from_secs(5),
            read_request: Duration::from_secs(1),
            write: Duration::from_secs(1),
            persistent_start_read_request: Duration::from_secs(1),
        })
        .keep_connection_alive()
    );

    (router, config)
}

pub struct Application;

impl AppBuilder for Application {
    type PathRouter = impl routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        picoserve::Router::new().route(
            "/",
            routing::get_service(File::html(include_str!("index.html"))),
        )
    }
}
