use bookworm::{cron_registry, observability};
use eyes_subscriber::ProcessIdentity;
use serde_json::Value;
use std::collections::BTreeSet;

fn references(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("query_id").and_then(Value::as_str) {
                ids.push(id.into());
            }
            for value in object.values() {
                references(value, ids);
            }
        }
        Value::Array(values) => {
            for value in values {
                references(value, ids);
            }
        }
        _ => {}
    }
}

#[test]
fn manifest_declares_only_enabled_crons_and_resolves_every_dashboard_card() {
    let registry = cron_registry();
    for enabled in [true, false] {
        let identity = ProcessIdentity::new(observability::ROLE);
        let manifest =
            observability::manifest(identity.clone(), enabled.then_some(&registry)).unwrap();
        assert_eq!(manifest.process_instance_id, Some(identity.instance_id()));
        assert_eq!(manifest.process_role.as_deref(), Some("bookworm"));
        assert_eq!(manifest.monitors.as_deref(), Some([].as_slice()));
        assert!(manifest.expected_process_roles.is_none());
        assert!(manifest.jobs.is_empty());
        assert_eq!(manifest.crons.len(), usize::from(enabled));
        if enabled {
            assert_eq!(manifest.crons[0].name, "weekly_reading_email");
            assert_eq!(
                manifest.crons[0].schedule,
                "CRON_TZ=America/New_York 0 0 18 * * Sun *"
            );
        }
        let metrics = manifest.metrics.as_ref().unwrap();
        assert_eq!(metrics.len(), 12);
        let ids: BTreeSet<_> = metrics.iter().map(|metric| metric.id.as_str()).collect();
        assert_eq!(ids.len(), 12);
        let dashboards = manifest.dashboards.as_ref().unwrap();
        assert_eq!(dashboards.len(), 1);
        let serialized = serde_json::to_value(dashboards).unwrap();
        let mut queries = vec![];
        references(&serialized, &mut queries);
        assert_eq!(queries.len(), 12);
        for query in queries {
            assert!(ids.contains(query.as_str()), "unresolved {query}");
        }
    }
}
