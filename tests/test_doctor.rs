use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use airgap::{config::Config, doctor, gmi::http_client};
use serde_json::json;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn config(server: &MockServer) -> Config {
    let mut config = Config::from_values(BTreeMap::new()).unwrap();
    config.agentbox_base_url = server.uri();
    config.maas_base_url = server.uri();
    config.agentbox_key = Some("synthetic-compute".into());
    config.maas_key = Some("synthetic-inference".into());
    config
}

#[tokio::test]
async fn doctor_checks_both_scopes_and_requested_model_without_compute_creation() {
    let server = MockServer::start().await;
    let config = config(&server);
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer synthetic-inference"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":config.model}]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/eligibility"))
        .and(header("authorization", "Bearer synthetic-compute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"eligible":true})))
        .expect(1)
        .mount(&server)
        .await;
    let report = doctor::check(&config, &http_client().unwrap()).await;
    assert!(report.maas_ok && report.compute_ok && report.model_available);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn missing_keys_and_rejected_or_missing_model_access_are_not_ready() {
    let server = MockServer::start().await;
    let mut config = config(&server);
    config.maas_key = None;
    config.agentbox_key = None;
    let report = doctor::check(&config, &http_client().unwrap()).await;
    assert!(!report.maas_ok && !report.compute_ok && !report.model_available);
    assert!(server.received_requests().await.unwrap().is_empty());
    config.maas_key = Some("synthetic-inference".into());
    config.agentbox_key = Some("synthetic-compute".into());
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[]})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/eligibility"))
        .respond_with(ResponseTemplate::new(403).set_body_string("private provider body"))
        .mount(&server)
        .await;
    let report = doctor::check(&config, &http_client().unwrap()).await;
    assert!(report.maas_ok && !report.model_available && !report.compute_ok);
    assert!(!format!("{report:?}").contains("private provider body"));
}

#[tokio::test]
async fn diagnostic_timeouts_run_concurrently_instead_of_waiting_twice() {
    let server = Arc::new(MockServer::start().await);
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(6))
                .set_body_json(json!({})),
        )
        .expect(2)
        .mount(&server)
        .await;
    let start = Instant::now();
    let report = doctor::check(&config(&server), &http_client().unwrap()).await;
    assert!(!report.maas_ok && !report.compute_ok);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "three-second requests must run concurrently"
    );
}
