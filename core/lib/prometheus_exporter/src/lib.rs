use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
use micro_config::configs::utils::Prometheus as PrometheusConfig;
use tokio::task::JoinHandle;

pub fn run_prometheus_exporter(config: PrometheusConfig, use_pushgateway: bool) -> JoinHandle<()> {
    // in seconds
    let default_latency_buckets = [0.001, 0.005, 0.025, 0.1, 0.25, 1.0, 5.0, 30.0, 120.0];
    let slow_latency_buckets = [
        0.33, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 180.0, 600.0, 1800.0, 3600.0,
    ];
    let prover_buckets = [
        1.0, 10.0, 20.0, 40.0, 60.0, 120.0, 240.0, 360.0, 600.0, 1800.0, 3600.0,
    ];

    let storage_interactions_per_call_buckets = [
        10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0,
    ];
    let vm_memory_per_call_buckets = [
        1000.0,
        10000.0,
        100000.0,
        500000.0,
        1000000.0,
        5000000.0,
        10000000.0,
        50000000.0,
        100000000.0,
        500000000.0,
        1000000000.0,
    ];
    let percents_buckets = [
        5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 120.0,
    ];
    let zero_to_one_buckets = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

    let builder = if use_pushgateway {
        let job_id = "micro-pushgateway";
        let namespace = std::env::var("POD_NAMESPACE").unwrap_or_else(|_| {
            vlog::warn!("Missing POD_NAMESPACE env");
            "UNKNOWN_NAMESPACE".to_string()
        });
        let pod = std::env::var("POD_NAME").unwrap_or_else(|_| {
            vlog::warn!("Missing POD_NAME env");
            "UNKNOWN_POD".to_string()
        });
        let endpoint = format!(
            "{}/metrics/job/{}/namespace/{}/pod/{}",
            config.pushgateway_url, job_id, namespace, pod
        );
        PrometheusBuilder::new()
            .with_push_gateway(endpoint.as_str(), config.push_interval())
            .unwrap()
    } else {
        let addr = ([0, 0, 0, 0], config.listener_port);
        PrometheusBuilder::new().with_http_listener(addr)
    };

    let (recorder, exporter) = builder
        .set_buckets(&default_latency_buckets)
        .unwrap()
        .set_buckets_for_metric(
            Matcher::Full("runtime_context.storage_interaction.amount".to_owned()),
            &storage_interactions_per_call_buckets,
        )
        .unwrap()
        .set_buckets_for_metric(
            Matcher::Full("runtime_context.storage_interaction.ratio".to_owned()),
            &zero_to_one_buckets,
        )
        .unwrap()
        .set_buckets_for_metric(
            Matcher::Prefix("runtime_context.memory".to_owned()),
            &vm_memory_per_call_buckets,
        )
        .unwrap()
        .set_buckets_for_metric(Matcher::Prefix("server.prover".to_owned()), &prover_buckets)
        .unwrap()
        .set_buckets_for_metric(
            Matcher::Prefix("server.witness_generator".to_owned()),
            &slow_latency_buckets,
        )
        .unwrap()
        .set_buckets_for_metric(Matcher::Prefix("vm.refund".to_owned()), &percents_buckets)
        .unwrap()
        .build()
        .expect("failed to install Prometheus recorder");

    metrics::set_boxed_recorder(Box::new(recorder)).expect("failed to set metrics recorder");

    tokio::spawn(async move {
        tokio::pin!(exporter);
        loop {
            tokio::select! {
                _ = &mut exporter => {}
            }
        }
    })
}
