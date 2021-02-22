use crate::{BackendResponse, Computation, COMPUTATION};
use json::JsonValue;
use std::convert::TryFrom;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub fn get_results() -> BackendResponse {
    let is_partial;
    let (data, elapsed) = {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            is_partial = cmp.thread.is_some();
            if let Some(classes) = &cmp.classifier {
                let mut result = None;
                for _ in 0..5 {
                    if let Some(data) = classes.try_export_result() {
                        result = Some(data);
                        break;
                    }
                    // wait a little - maybe the lock will become free
                    std::thread::sleep(Duration::new(1, 0));
                }
                if let Some(result) = result {
                    (
                        result,
                        cmp.end_timestamp().map(|t| t - cmp.start_timestamp()),
                    )
                } else {
                    return BackendResponse::err(
                        &"Classification running. Cannot export components right now.".to_string(),
                    );
                }
            } else {
                return BackendResponse::err(&"Results not available yet.".to_string());
            }
        } else {
            return BackendResponse::err(&"No results available.".to_string());
        }
    };
    let lines: Vec<JsonValue> = data
        .iter()
        .map(|(c, p)| {
            object! {
                "sat_count": p.cardinality(),
                "phenotype": c.to_vec()
                                .into_iter()
                                .map(|b| format!("{:?}", b))
                                .collect::<Vec<String>>(),
            }
        })
        .collect();

    let elapsed = if let Some(e) = elapsed { e } else { 0 };
    let elapsed = u64::try_from(elapsed).unwrap_or(0);

    let result = object! {
        "isPartial": is_partial,
        "elapsed": elapsed,
        "data": lines
    };

    return BackendResponse::ok_json(result);
}
