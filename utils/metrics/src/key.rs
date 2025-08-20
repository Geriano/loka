use std::sync::OnceLock;

use dashmap::DashMap;

static KEYS: OnceLock<DashMap<metrics::Key, String>> = OnceLock::new();

pub(crate) fn to_string(key: &metrics::Key) -> String {
    let keys = KEYS.get_or_init(|| Default::default());

    keys.entry(key.clone())
        .or_insert_with(|| {
            if key.labels().len() > 0 {
                let labels = key
                    .labels()
                    .map(|label| format!(r#"{}="{}""#, label.key(), label.value()))
                    .collect::<Vec<String>>();

                format!("{}{{{}}}", key.name(), labels.join(","))
            } else {
                key.name().to_string()
            }
        })
        .value()
        .to_string()
}
