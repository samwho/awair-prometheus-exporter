use log::LevelFilter;
use once_cell::sync::OnceCell;
use std::convert::Infallible;

use lazy_static::lazy_static;
use log::{debug, error};
use prometheus::{
    default_registry, register_gauge, register_int_gauge, Encoder, Gauge, IntGauge, TextEncoder,
};
use reqwest::get;
use reqwest::Url;
use serde_derive::Deserialize;
use structopt::StructOpt;
use warp::http::{Response, StatusCode};
use warp::Filter;

lazy_static! {
    static ref OPT: OnceCell<Opt> = OnceCell::new();
    static ref SCORE: IntGauge = register_int_gauge!("awair_score", "Air quality score").unwrap();
    static ref DEW_POINT: Gauge = register_gauge!("awair_dew_point", "Dew point").unwrap();
    static ref TEMP: Gauge = register_gauge!("awair_temp", "Temperature").unwrap();
    static ref HUMID: Gauge = register_gauge!("awair_humid", "Humidity").unwrap();
    static ref ABS_HUMID: Gauge = register_gauge!("awair_abs_humid", "Absolute humidity").unwrap();
    static ref CO2: IntGauge = register_int_gauge!("awair_co2", "CO2").unwrap();
    static ref CO2_EST: IntGauge = register_int_gauge!("awair_co2_est", "Estimated CO2").unwrap();
    static ref CO2_EST_BASELINE: IntGauge =
        register_int_gauge!("awair_co2_est_baseline", "Estimated CO2 baseline").unwrap();
    static ref VOC: IntGauge = register_int_gauge!("awair_voc", "VOC").unwrap();
    static ref VOC_BASELINE: IntGauge =
        register_int_gauge!("awair_voc_baseline", "VOC baseline").unwrap();
    static ref VOC_H2_RAW: IntGauge =
        register_int_gauge!("awair_voc_h2_raw", "VOC H2 raw").unwrap();
    static ref VOC_ETHANOL_RAW: IntGauge =
        register_int_gauge!("awair_voc_ethanol_raw", "VOC ethanol raw").unwrap();
    static ref PM25: IntGauge = register_int_gauge!("awair_pm25", "PM25").unwrap();
    static ref PM10_EST: IntGauge =
        register_int_gauge!("awair_pm10_est", "Estimated PM10").unwrap();
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "awair-prometheus-exporter")]
struct Opt {
    #[structopt(long)]
    target: Url,

    #[structopt(long, default_value = "8888")]
    metrics_port: u16,
}

#[derive(Deserialize, Debug)]
struct AirData {
    timestamp: String,
    score: i64,
    dew_point: f64,
    temp: f64,
    humid: f64,
    abs_humid: f64,
    co2: i64,
    co2_est: i64,
    co2_est_baseline: i64,
    voc: i64,
    voc_baseline: i64,
    voc_h2_raw: i64,
    voc_ethanol_raw: i64,
    pm25: i64,
    pm10_est: i64,
}

async fn poll() -> Result<(), Box<dyn std::error::Error>> {
    let opt = OPT.get().unwrap();
    let url = opt.target.join("/air-data/latest")?;

    debug!("sending request to Awair device: {}", url);
    let data: AirData = get(url).await?.json().await?;
    debug!("got data: {:?}", data);

    SCORE.set(data.score);
    DEW_POINT.set(data.dew_point);
    TEMP.set(data.temp);
    HUMID.set(data.humid);
    ABS_HUMID.set(data.abs_humid);
    CO2.set(data.co2);
    CO2_EST.set(data.co2_est);
    CO2_EST_BASELINE.set(data.co2_est_baseline);
    VOC.set(data.voc);
    VOC_BASELINE.set(data.voc_baseline);
    VOC_H2_RAW.set(data.voc_h2_raw);
    VOC_ETHANOL_RAW.set(data.voc_ethanol_raw);
    PM25.set(data.pm25);
    PM10_EST.set(data.pm10_est);

    Ok(())
}

async fn metrics() -> Result<impl warp::Reply, Infallible> {
    if let Err(e) = poll().await {
        error!("error during metrics poll: {}", e);
        return Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body("".to_owned()));
    }

    let mut buf = vec![];
    let registry = default_registry();
    let encoder = TextEncoder::new();
    let metrics = registry.gather();
    encoder.encode(&metrics, &mut buf).unwrap();

    let body = String::from_utf8(buf).unwrap();
    Ok(Response::builder().body(body))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let opt = Opt::from_args();
    OPT.set(opt.clone()).unwrap();

    let metrics = warp::path!("metrics")
        .and_then(metrics)
        .with(warp::log("warp::server"));

    warp::serve(metrics)
        .run(([0, 0, 0, 0], opt.metrics_port))
        .await;
    Ok(())
}
