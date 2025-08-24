// Connection metrics constants
pub const STRATUM_TOTAL_CONNECTIONS_HELP: &str =
    "# HELP stratum_total_connections Total number of connections established";
pub const STRATUM_TOTAL_CONNECTIONS_TYPE: &str = "# TYPE stratum_total_connections counter";
pub const STRATUM_ACTIVE_CONNECTIONS_HELP: &str =
    "# HELP stratum_active_connections Current number of active connections";
pub const STRATUM_ACTIVE_CONNECTIONS_TYPE: &str = "# TYPE stratum_active_connections gauge";

pub const STRATUM_CONNECTION_ERRORS_HELP: &str =
    "# HELP stratum_connection_errors Total connection errors";
pub const STRATUM_CONNECTION_ERRORS_TYPE: &str = "# TYPE stratum_connection_errors counter";

// Connection lifecycle metrics
pub const STRATUM_CONNECTION_DURATION_AVG_HELP: &str =
    "# HELP stratum_connection_duration_avg_seconds Average connection duration";
pub const STRATUM_CONNECTION_DURATION_AVG_TYPE: &str =
    "# TYPE stratum_connection_duration_avg_seconds gauge";
pub const STRATUM_CONNECTION_DURATION_MAX_HELP: &str =
    "# HELP stratum_connection_duration_max_seconds Maximum connection duration";
pub const STRATUM_CONNECTION_DURATION_MAX_TYPE: &str =
    "# TYPE stratum_connection_duration_max_seconds gauge";
pub const STRATUM_CONNECTION_DURATION_MIN_HELP: &str =
    "# HELP stratum_connection_duration_min_seconds Minimum connection duration";
pub const STRATUM_CONNECTION_DURATION_MIN_TYPE: &str =
    "# TYPE stratum_connection_duration_min_seconds gauge";

pub const STRATUM_IDLE_TIME_HELP: &str = "# HELP stratum_idle_time_seconds Current idle time gauge";
pub const STRATUM_IDLE_TIME_TYPE: &str = "# TYPE stratum_idle_time_seconds gauge";

pub const STRATUM_RECONNECTION_ATTEMPTS_HELP: &str =
    "# HELP stratum_reconnection_attempts_total Total reconnection attempts";
pub const STRATUM_RECONNECTION_ATTEMPTS_TYPE: &str =
    "# TYPE stratum_reconnection_attempts_total counter";

pub const STRATUM_CONNECTION_ESTABLISHED_HELP: &str =
    "# HELP stratum_connection_established_total Total connections established";
pub const STRATUM_CONNECTION_ESTABLISHED_TYPE: &str =
    "# TYPE stratum_connection_established_total counter";

pub const STRATUM_CONNECTION_CLOSED_HELP: &str =
    "# HELP stratum_connection_closed_total Total connections closed";
pub const STRATUM_CONNECTION_CLOSED_TYPE: &str = "# TYPE stratum_connection_closed_total counter";

// Protocol detection metrics
pub const STRATUM_HTTP_REQUESTS_HELP: &str =
    "# HELP stratum_http_requests_total Total HTTP requests detected";
pub const STRATUM_HTTP_REQUESTS_TYPE: &str = "# TYPE stratum_http_requests_total counter";

pub const STRATUM_STRATUM_REQUESTS_HELP: &str =
    "# HELP stratum_stratum_requests_total Total Stratum requests detected";
pub const STRATUM_STRATUM_REQUESTS_TYPE: &str = "# TYPE stratum_stratum_requests_total counter";

pub const STRATUM_PROTOCOL_DETECTION_FAILURES_HELP: &str =
    "# HELP stratum_protocol_detection_failures_total Protocol detection failures";
pub const STRATUM_PROTOCOL_DETECTION_FAILURES_TYPE: &str =
    "# TYPE stratum_protocol_detection_failures_total counter";

pub const STRATUM_PROTOCOL_DETECTION_SUCCESSES_HELP: &str =
    "# HELP stratum_protocol_detection_successes_total Protocol detection successes";
pub const STRATUM_PROTOCOL_DETECTION_SUCCESSES_TYPE: &str =
    "# TYPE stratum_protocol_detection_successes_total counter";

// Mining operation metrics
pub const STRATUM_SHARE_SUBMISSIONS_HELP: &str =
    "# HELP stratum_share_submissions_total Total share submissions";
pub const STRATUM_SHARE_SUBMISSIONS_TYPE: &str = "# TYPE stratum_share_submissions_total counter";

pub const STRATUM_SHARE_ACCEPTANCE_RATE_HELP: &str =
    "# HELP stratum_share_acceptance_rate Share acceptance rate";
pub const STRATUM_SHARE_ACCEPTANCE_RATE_TYPE: &str = "# TYPE stratum_share_acceptance_rate gauge";

pub const STRATUM_DIFFICULTY_ADJUSTMENTS_HELP: &str =
    "# HELP stratum_difficulty_adjustments_total Total difficulty adjustments";
pub const STRATUM_DIFFICULTY_ADJUSTMENTS_TYPE: &str =
    "# TYPE stratum_difficulty_adjustments_total counter";

pub const STRATUM_CURRENT_DIFFICULTY_HELP: &str =
    "# HELP stratum_current_difficulty Current mining difficulty";
pub const STRATUM_CURRENT_DIFFICULTY_TYPE: &str = "# TYPE stratum_current_difficulty gauge";

// Error categorization metrics
pub const STRATUM_NETWORK_ERRORS_HELP: &str =
    "# HELP stratum_network_errors_total Network-related errors";
pub const STRATUM_NETWORK_ERRORS_TYPE: &str = "# TYPE stratum_network_errors_total counter";

pub const STRATUM_AUTHENTICATION_FAILURES_HELP: &str =
    "# HELP stratum_authentication_failures_total Authentication failures";
pub const STRATUM_AUTHENTICATION_FAILURES_TYPE: &str =
    "# TYPE stratum_authentication_failures_total counter";

pub const STRATUM_TIMEOUT_ERRORS_HELP: &str = "# HELP stratum_timeout_errors_total Timeout errors";
pub const STRATUM_TIMEOUT_ERRORS_TYPE: &str = "# TYPE stratum_timeout_errors_total counter";

// Resource utilization metrics
pub const STRATUM_CPU_UTILIZATION_HELP: &str =
    "# HELP stratum_cpu_utilization_current_percent Current CPU utilization percentage";
pub const STRATUM_CPU_UTILIZATION_TYPE: &str =
    "# TYPE stratum_cpu_utilization_current_percent gauge";

pub const STRATUM_MEMORY_USAGE_AVG_HELP: &str =
    "# HELP stratum_memory_usage_per_connection_avg_mb Average memory usage per connection";
pub const STRATUM_MEMORY_USAGE_AVG_TYPE: &str =
    "# TYPE stratum_memory_usage_per_connection_avg_mb gauge";

// Additional protocol detection constants
pub const STRATUM_PROTOCOL_CONVERSION_SUCCESS_RATE_HELP: &str =
    "# HELP stratum_protocol_conversion_success_rate Protocol conversion success rate";
pub const STRATUM_PROTOCOL_CONVERSION_SUCCESS_RATE_TYPE: &str =
    "# TYPE stratum_protocol_conversion_success_rate gauge";

pub const STRATUM_PROTOCOL_CONVERSION_ERRORS_HELP: &str =
    "# HELP stratum_protocol_conversion_errors_total Protocol conversion errors";
pub const STRATUM_PROTOCOL_CONVERSION_ERRORS_TYPE: &str =
    "# TYPE stratum_protocol_conversion_errors_total counter";

pub const STRATUM_HTTP_CONNECT_REQUESTS_HELP: &str =
    "# HELP stratum_http_connect_requests_total HTTP CONNECT requests";
pub const STRATUM_HTTP_CONNECT_REQUESTS_TYPE: &str =
    "# TYPE stratum_http_connect_requests_total counter";

pub const STRATUM_DIRECT_STRATUM_CONNECTIONS_HELP: &str =
    "# HELP stratum_direct_stratum_connections_total Direct Stratum connections";
pub const STRATUM_DIRECT_STRATUM_CONNECTIONS_TYPE: &str =
    "# TYPE stratum_direct_stratum_connections_total counter";

// Additional mining operation constants
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_HELP: &str =
    "# HELP stratum_job_distribution_latency_avg_seconds Average job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_avg_seconds gauge";

pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_HELP: &str =
    "# HELP stratum_job_distribution_latency_max_seconds Maximum job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_max_seconds gauge";

pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_HELP: &str =
    "# HELP stratum_job_distribution_latency_min_seconds Minimum job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_min_seconds gauge";

pub const STRATUM_SHARES_PER_MINUTE_HELP: &str =
    "# HELP stratum_shares_per_minute Current shares per minute";
pub const STRATUM_SHARES_PER_MINUTE_TYPE: &str = "# TYPE stratum_shares_per_minute gauge";

pub const STRATUM_STALE_SHARES_HELP: &str = "# HELP stratum_stale_shares_total Total stale shares";
pub const STRATUM_STALE_SHARES_TYPE: &str = "# TYPE stratum_stale_shares_total counter";

pub const STRATUM_DUPLICATE_SHARES_HELP: &str =
    "# HELP stratum_duplicate_shares_total Total duplicate shares";
pub const STRATUM_DUPLICATE_SHARES_TYPE: &str = "# TYPE stratum_duplicate_shares_total counter";

// Additional error constants
pub const STRATUM_PROTOCOL_PARSE_ERRORS_HELP: &str =
    "# HELP stratum_protocol_parse_errors Protocol parsing errors";
pub const STRATUM_PROTOCOL_PARSE_ERRORS_TYPE: &str = "# TYPE stratum_protocol_parse_errors counter";

pub const STRATUM_PROTOCOL_VERSION_ERRORS_HELP: &str =
    "# HELP stratum_protocol_version_errors Protocol version errors";
pub const STRATUM_PROTOCOL_VERSION_ERRORS_TYPE: &str =
    "# TYPE stratum_protocol_version_errors counter";

pub const STRATUM_PROTOCOL_MESSAGE_ERRORS_HELP: &str =
    "# HELP stratum_protocol_message_errors Protocol message errors";
pub const STRATUM_PROTOCOL_MESSAGE_ERRORS_TYPE: &str =
    "# TYPE stratum_protocol_message_errors counter";

pub const STRATUM_VALIDATION_ERRORS_HELP: &str =
    "# HELP stratum_validation_errors Validation errors";
pub const STRATUM_VALIDATION_ERRORS_TYPE: &str = "# TYPE stratum_validation_errors counter";

pub const STRATUM_SECURITY_VIOLATION_ERRORS_HELP: &str =
    "# HELP stratum_security_violation_errors Security violation errors";
pub const STRATUM_SECURITY_VIOLATION_ERRORS_TYPE: &str =
    "# TYPE stratum_security_violation_errors counter";

pub const STRATUM_RESOURCE_EXHAUSTION_ERRORS_HELP: &str =
    "# HELP stratum_resource_exhaustion_errors Resource exhaustion errors";
pub const STRATUM_RESOURCE_EXHAUSTION_ERRORS_TYPE: &str =
    "# TYPE stratum_resource_exhaustion_errors counter";

pub const STRATUM_INTERNAL_ERRORS_HELP: &str = "# HELP stratum_internal_errors Internal errors";
pub const STRATUM_INTERNAL_ERRORS_TYPE: &str = "# TYPE stratum_internal_errors counter";

// Additional resource utilization constants
pub const STRATUM_MEMORY_USAGE_MAX_HELP: &str =
    "# HELP stratum_memory_usage_per_connection_max_mb Maximum memory usage per connection";
pub const STRATUM_MEMORY_USAGE_MAX_TYPE: &str =
    "# TYPE stratum_memory_usage_per_connection_max_mb gauge";

pub const STRATUM_MEMORY_USAGE_MIN_HELP: &str =
    "# HELP stratum_memory_usage_per_connection_min_mb Minimum memory usage per connection";
pub const STRATUM_MEMORY_USAGE_MIN_TYPE: &str =
    "# TYPE stratum_memory_usage_per_connection_min_mb gauge";

pub const STRATUM_NETWORK_BANDWIDTH_RX_HELP: &str =
    "# HELP stratum_network_bandwidth_rx_bytes_per_sec Network bandwidth receive rate";
pub const STRATUM_NETWORK_BANDWIDTH_RX_TYPE: &str =
    "# TYPE stratum_network_bandwidth_rx_bytes_per_sec gauge";

pub const STRATUM_NETWORK_BANDWIDTH_TX_HELP: &str =
    "# HELP stratum_network_bandwidth_tx_bytes_per_sec Network bandwidth transmit rate";
pub const STRATUM_NETWORK_BANDWIDTH_TX_TYPE: &str =
    "# TYPE stratum_network_bandwidth_tx_bytes_per_sec gauge";

pub const STRATUM_CONNECTION_MEMORY_TOTAL_HELP: &str =
    "# HELP stratum_connection_memory_total_bytes Total memory used by connections";
pub const STRATUM_CONNECTION_MEMORY_TOTAL_TYPE: &str =
    "# TYPE stratum_connection_memory_total_bytes gauge";

pub const STRATUM_CONNECTION_MEMORY_PEAK_HELP: &str =
    "# HELP stratum_connection_memory_peak_bytes Peak memory used by connections";
pub const STRATUM_CONNECTION_MEMORY_PEAK_TYPE: &str =
    "# TYPE stratum_connection_memory_peak_bytes gauge";

// Additional metric variations and legacy metrics
pub const STRATUM_CONNECTION_DURATION_AVG_MS_HELP: &str =
    "# HELP stratum_connection_duration_avg_ms Average connection duration";
pub const STRATUM_CONNECTION_DURATION_AVG_MS_TYPE: &str =
    "# TYPE stratum_connection_duration_avg_ms gauge";

pub const STRATUM_IDLE_TIME_GAUGE_MS_HELP: &str =
    "# HELP stratum_idle_time_gauge_ms Current connection idle time";
pub const STRATUM_IDLE_TIME_GAUGE_MS_TYPE: &str = "# TYPE stratum_idle_time_gauge_ms gauge";

pub const STRATUM_RECONNECTION_ATTEMPTS_COUNTER_HELP: &str =
    "# HELP stratum_reconnection_attempts_counter Total reconnection attempts";
pub const STRATUM_RECONNECTION_ATTEMPTS_COUNTER_TYPE: &str =
    "# TYPE stratum_reconnection_attempts_counter counter";

pub const STRATUM_CONNECTION_ESTABLISHED_COUNTER_HELP: &str =
    "# HELP stratum_connection_established_counter Connections established";
pub const STRATUM_CONNECTION_ESTABLISHED_COUNTER_TYPE: &str =
    "# TYPE stratum_connection_established_counter counter";

pub const STRATUM_CONNECTION_CLOSED_COUNTER_HELP: &str =
    "# HELP stratum_connection_closed_counter Connections closed";
pub const STRATUM_CONNECTION_CLOSED_COUNTER_TYPE: &str =
    "# TYPE stratum_connection_closed_counter counter";

pub const STRATUM_PROTOCOL_CONVERSION_AVG_SUCCESS_RATE_HELP: &str =
    "# HELP stratum_protocol_conversion_avg_success_rate Average protocol conversion success rate";
pub const STRATUM_PROTOCOL_CONVERSION_AVG_SUCCESS_RATE_TYPE: &str =
    "# TYPE stratum_protocol_conversion_avg_success_rate gauge";

pub const STRATUM_SHARE_ACCEPTANCE_AVG_RATE_HELP: &str =
    "# HELP stratum_share_acceptance_avg_rate Average share acceptance rate";
pub const STRATUM_SHARE_ACCEPTANCE_AVG_RATE_TYPE: &str =
    "# TYPE stratum_share_acceptance_avg_rate gauge";

pub const STRATUM_DIFFICULTY_ADJUSTMENTS_COUNTER_HELP: &str =
    "# HELP stratum_difficulty_adjustments_counter Difficulty adjustments counter";
pub const STRATUM_DIFFICULTY_ADJUSTMENTS_COUNTER_TYPE: &str =
    "# TYPE stratum_difficulty_adjustments_counter counter";

pub const STRATUM_SHARES_PER_MINUTE_GAUGE_HELP: &str =
    "# HELP stratum_shares_per_minute_gauge Shares per minute gauge";
pub const STRATUM_SHARES_PER_MINUTE_GAUGE_TYPE: &str =
    "# TYPE stratum_shares_per_minute_gauge gauge";

pub const STRATUM_STALE_SHARES_COUNTER_HELP: &str =
    "# HELP stratum_stale_shares_counter Stale shares counter";
pub const STRATUM_STALE_SHARES_COUNTER_TYPE: &str = "# TYPE stratum_stale_shares_counter counter";

pub const STRATUM_DUPLICATE_SHARES_COUNTER_HELP: &str =
    "# HELP stratum_duplicate_shares_counter Duplicate shares counter";
pub const STRATUM_DUPLICATE_SHARES_COUNTER_TYPE: &str =
    "# TYPE stratum_duplicate_shares_counter counter";

// Job distribution latency variations
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_MS_HELP: &str =
    "# HELP stratum_job_distribution_latency_avg_ms Average job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_MS_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_avg_ms gauge";

pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_MS_HELP: &str =
    "# HELP stratum_job_distribution_latency_max_ms Maximum job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_MS_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_max_ms gauge";

pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_MS_HELP: &str =
    "# HELP stratum_job_distribution_latency_min_ms Minimum job distribution latency";
pub const STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_MS_TYPE: &str =
    "# TYPE stratum_job_distribution_latency_min_ms gauge";

// Error metric variations with _total suffix
pub const STRATUM_TIMEOUT_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_timeout_errors_total Timeout errors";
pub const STRATUM_TIMEOUT_ERRORS_TOTAL_TYPE: &str = "# TYPE stratum_timeout_errors_total counter";

pub const STRATUM_PROTOCOL_PARSE_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_protocol_parse_errors_total Protocol parsing errors";
pub const STRATUM_PROTOCOL_PARSE_ERRORS_TOTAL_TYPE: &str =
    "# TYPE stratum_protocol_parse_errors_total counter";

pub const STRATUM_VALIDATION_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_validation_errors_total Validation errors";
pub const STRATUM_VALIDATION_ERRORS_TOTAL_TYPE: &str =
    "# TYPE stratum_validation_errors_total counter";

pub const STRATUM_SECURITY_VIOLATION_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_security_violation_errors_total Security violation errors";
pub const STRATUM_SECURITY_VIOLATION_ERRORS_TOTAL_TYPE: &str =
    "# TYPE stratum_security_violation_errors_total counter";

pub const STRATUM_RESOURCE_EXHAUSTION_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_resource_exhaustion_errors_total Resource exhaustion errors";
pub const STRATUM_RESOURCE_EXHAUSTION_ERRORS_TOTAL_TYPE: &str =
    "# TYPE stratum_resource_exhaustion_errors_total counter";

pub const STRATUM_INTERNAL_ERRORS_TOTAL_HELP: &str =
    "# HELP stratum_internal_errors_total Internal errors";
pub const STRATUM_INTERNAL_ERRORS_TOTAL_TYPE: &str = "# TYPE stratum_internal_errors_total counter";

// Additional resource utilization metrics
pub const STRATUM_MEMORY_EFFICIENCY_RATIO_HELP: &str =
    "# HELP stratum_memory_efficiency_ratio Memory efficiency ratio";
pub const STRATUM_MEMORY_EFFICIENCY_RATIO_TYPE: &str =
    "# TYPE stratum_memory_efficiency_ratio gauge";

pub const STRATUM_RESOURCE_PRESSURE_EVENTS_HELP: &str =
    "# HELP stratum_resource_pressure_events Resource pressure events";
pub const STRATUM_RESOURCE_PRESSURE_EVENTS_TYPE: &str =
    "# TYPE stratum_resource_pressure_events counter";

// Basic protocol metrics
pub const STRATUM_MESSAGES_RECEIVED_HELP: &str =
    "# HELP stratum_messages_received Total messages received";
pub const STRATUM_MESSAGES_RECEIVED_TYPE: &str = "# TYPE stratum_messages_received counter";

pub const STRATUM_SUBMISSIONS_RECEIVED_HELP: &str =
    "# HELP stratum_submissions_received Total submissions received";
pub const STRATUM_SUBMISSIONS_RECEIVED_TYPE: &str = "# TYPE stratum_submissions_received counter";

pub const STRATUM_SUBMISSIONS_ACCEPTED_HELP: &str =
    "# HELP stratum_submissions_accepted Total submissions accepted";
pub const STRATUM_SUBMISSIONS_ACCEPTED_TYPE: &str = "# TYPE stratum_submissions_accepted counter";

// Authentication metrics
pub const STRATUM_AUTH_SUCCESSES_HELP: &str =
    "# HELP stratum_auth_successes Authentication successes";
pub const STRATUM_AUTH_SUCCESSES_TYPE: &str = "# TYPE stratum_auth_successes counter";

pub const STRATUM_AUTH_FAILURES_HELP: &str = "# HELP stratum_auth_failures Authentication failures";
pub const STRATUM_AUTH_FAILURES_TYPE: &str = "# TYPE stratum_auth_failures counter";

pub const STRATUM_AUTH_ATTEMPTS_HELP: &str =
    "# HELP stratum_auth_attempts Total authentication attempts";
pub const STRATUM_AUTH_ATTEMPTS_TYPE: &str = "# TYPE stratum_auth_attempts counter";

// Legacy submission metrics
pub const STRATUM_TOTAL_SUBMISSIONS_HELP: &str =
    "# HELP stratum_total_submissions Total submissions received";
pub const STRATUM_TOTAL_SUBMISSIONS_TYPE: &str = "# TYPE stratum_total_submissions counter";

pub const STRATUM_ACCEPTED_SUBMISSIONS_HELP: &str =
    "# HELP stratum_accepted_submissions Total submissions accepted";
pub const STRATUM_ACCEPTED_SUBMISSIONS_TYPE: &str = "# TYPE stratum_accepted_submissions counter";

pub const STRATUM_REJECTED_SUBMISSIONS_HELP: &str =
    "# HELP stratum_rejected_submissions Total submissions rejected";
pub const STRATUM_REJECTED_SUBMISSIONS_TYPE: &str = "# TYPE stratum_rejected_submissions counter";

// Legacy security metrics
pub const STRATUM_SECURITY_VIOLATIONS_HELP: &str =
    "# HELP stratum_security_violations Total security violations";
pub const STRATUM_SECURITY_VIOLATIONS_TYPE: &str = "# TYPE stratum_security_violations counter";

pub const STRATUM_RATE_LIMIT_HITS_HELP: &str =
    "# HELP stratum_rate_limit_hits Total rate limit hits";
pub const STRATUM_RATE_LIMIT_HITS_TYPE: &str = "# TYPE stratum_rate_limit_hits counter";

// Alternative constant names for duplicate protocol detection metrics
pub const STRATUM_HTTP_CONNECT_REQUESTS_ALT_HELP: &str =
    "# HELP stratum_http_connect_requests HTTP CONNECT requests";
pub const STRATUM_HTTP_CONNECT_REQUESTS_ALT_TYPE: &str =
    "# TYPE stratum_http_connect_requests counter";

pub const STRATUM_DIRECT_STRATUM_CONNECTIONS_ALT_HELP: &str =
    "# HELP stratum_direct_stratum_connections Direct Stratum connections";
pub const STRATUM_DIRECT_STRATUM_CONNECTIONS_ALT_TYPE: &str =
    "# TYPE stratum_direct_stratum_connections counter";

// User-specific metrics constants
pub const STRATUM_USER_TOTAL_SUBMISSIONS_HELP: &str =
    "# HELP stratum_user_total_submissions User submissions received";
pub const STRATUM_USER_TOTAL_SUBMISSIONS_TYPE: &str = "# TYPE stratum_user_total_submissions gauge";

pub const STRATUM_USER_ACCEPTED_SUBMISSIONS_HELP: &str =
    "# HELP stratum_user_accepted_submissions User submissions accepted";
pub const STRATUM_USER_ACCEPTED_SUBMISSIONS_TYPE: &str =
    "# TYPE stratum_user_accepted_submissions gauge";

pub const STRATUM_USER_REJECTED_SUBMISSIONS_HELP: &str =
    "# HELP stratum_user_rejected_submissions User submissions rejected";
pub const STRATUM_USER_REJECTED_SUBMISSIONS_TYPE: &str =
    "# TYPE stratum_user_rejected_submissions gauge";

pub const STRATUM_USER_HASHRATE_HELP: &str = "# HELP stratum_user_hashrate User hashrate estimate";
pub const STRATUM_USER_HASHRATE_TYPE: &str = "# TYPE stratum_user_hashrate gauge";

pub const STRATUM_USER_CONNECTIONS_HELP: &str =
    "# HELP stratum_user_connections User connection count";
pub const STRATUM_USER_CONNECTIONS_TYPE: &str = "# TYPE stratum_user_connections gauge";

pub const STRATUM_USER_MESSAGES_RECEIVED_HELP: &str =
    "# HELP stratum_user_messages_received User messages received";
pub const STRATUM_USER_MESSAGES_RECEIVED_TYPE: &str = "# TYPE stratum_user_messages_received gauge";

pub const STRATUM_USER_MESSAGES_SENT_HELP: &str =
    "# HELP stratum_user_messages_sent User messages sent";
pub const STRATUM_USER_MESSAGES_SENT_TYPE: &str = "# TYPE stratum_user_messages_sent gauge";
