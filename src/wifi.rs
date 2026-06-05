use defmt::{error, info, warn};
use embassy_net::{Config as NetConfig, DhcpConfig, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, sta::StationConfig};

use crate::mk_static;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub async fn start_wifi(
    wifi_peripheral: esp_hal::peripherals::WIFI<'static>,
    rng: Rng,
    spawner: &embassy_executor::Spawner,
) -> Stack<'static> {
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );
    let wifi_controller = esp_radio::wifi::WifiController::new(
        wifi_peripheral,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .unwrap();
    let interfaces_station = esp_radio::wifi::Interface::station();

    let net_config = NetConfig::dhcpv4(DhcpConfig::default());
    let wifi_controller = mk_static!(WifiController<'static>, wifi_controller);
    let resources = mk_static!(StackResources<4>, StackResources::<4>::new());
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
    let (stack, runner) = embassy_net::new(interfaces_station, net_config, resources, net_seed);

    spawner.spawn(connection_task(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    info!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    stack
}

#[embassy_executor::task]
async fn connection_task(controller: &'static mut WifiController<'static>) {
    loop {
        if !controller.is_connected() {
            info!("Connecting...");
            match controller.connect_async().await {
                Ok(info) => info!("Connected! {:?}", info),
                Err(e) => {
                    error!("Connect failed: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                }
            }
        } else {
            match controller.wait_for_disconnect_async().await {
                Ok(info) => info!("Disconnected: {:?}", info),
                Err(e) => warn!("Disconnect wait error: {:?}", e),
            }
            Timer::after(Duration::from_secs(5)).await;
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}
