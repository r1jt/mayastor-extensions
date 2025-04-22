/// Defines the etcd pagination limit.
pub(crate) const ETCD_PAGED_LIMIT: i64 = 1000;

/// Defines the name of mayastor-io container(dataplane container)
pub(crate) const DATA_PLANE_CONTAINER_NAME: &str = "io-engine";

/// Defines the logging label(key-value pair) on services.
pub(crate) fn logging_label_selector() -> String {
    format!("{}=true", ::constants::loki_logging_key())
}
