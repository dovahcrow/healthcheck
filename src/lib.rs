use axum::http::StatusCode;
use axum::{routing::get, Json, Router};
use once_cell::sync::OnceCell;
use prometheus::{gather, Encoder, TextEncoder};
use prometheus::{register_int_gauge, IntGauge};
use serde_json::Value;
use std::pin::Pin;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::RwLock;
use tokio::time::{sleep, Instant, Sleep};

static INSTANCE: OnceCell<HealthCheck> = OnceCell::new();
static HEALTHCHECK_METRIC: OnceCell<IntGauge> = OnceCell::new();

pub struct HealthCheck {
    ttl: Duration,
    timer: RwLock<Sleep>,
}

impl HealthCheck {
    pub fn init(ttl: Duration, port: u16) {
        INSTANCE.get_or_init(|| HealthCheck::new(ttl, port));
    }

    pub fn get() -> &'static Self {
        INSTANCE.get().unwrap()
    }

    fn new(ttl: Duration, port: u16) -> Self {
        let app = Router::new()
            .route("/healthcheck", get(Self::healthcheck))
            .route("/healthcheck/metrics", get(Self::prometheus));

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tokio::spawn(axum::Server::bind(&addr).serve(app.into_make_service()));

        tokio::spawn(async {
            loop {
                sleep(Duration::from_secs(1)).await;

                let hc = HealthCheck::get();

                let metric = HEALTHCHECK_METRIC.get_or_init(|| {
                    register_int_gauge!("healthcheck", "Whether the service is healthy").unwrap()
                });

                if hc.timer.read().await.is_elapsed() {
                    metric.set(0);
                } else {
                    metric.set(1);
                }
            }
        });

        Self {
            ttl,
            timer: RwLock::new(sleep(ttl)),
        }
    }

    pub async fn healthy() {
        let hc = Self::get();
        let mut sleep = hc.timer.write().await;

        unsafe { Pin::new_unchecked(&mut *sleep) }
            .reset(Instant::from_std(std::time::Instant::now() + hc.ttl))
    }

    async fn healthcheck() -> (StatusCode, Json<Value>) {
        let hc = Self::get();

        if hc.timer.read().await.is_elapsed() {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Value::String("unhealthy".into())),
            )
        } else {
            (StatusCode::OK, Json(Value::String("healthy".into())))
        }
    }

    async fn prometheus() -> Vec<u8> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        buffer
    }
}
