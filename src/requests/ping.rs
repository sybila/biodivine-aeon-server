use crate::{ArcComputation, BackendResponse};
use std::sync::atomic::Ordering;

pub fn ping(cmp: ArcComputation) -> BackendResponse {
    let mut response = object! {
        "timestamp" => json::Null,          // if there is some computation (not necessarily running, this is the time when it started
        "is_cancelled" => false,            // true if the computation has been canceled
        "running" => false,                 // true if the computation thread is still alive
        "progress" => "unknown".to_string(),// arbitrary progress string
        "error" => json::Null,              // arbitrary error string - currently not really used
        "num_classes" => json::Null         // number of discovered classes so far
    };
    {
        // Read data from current computation if available...
        let cmp = cmp.read().unwrap();
        if let Some(computation) = &*cmp {
            response["timestamp"] = (computation.start_timestamp() as u64).into();
            response["is_cancelled"] = computation.is_cancelled.load(Ordering::SeqCst).into();
            if let Some(progress) = &computation.progress {
                response["progress"] = progress.get_percent_string().into();
            }
            response["is_running"] = computation.thread.is_some().into();
            if let Some(error) = &computation.error {
                response["error"] = error.clone().into();
            }
            if let Some(classes) = computation
                .classifier
                .as_ref()
                .map(|c| c.try_get_num_classes())
            {
                response["num_classes"] = classes.into();
            }
        }
    }
    return BackendResponse::ok(&response.to_string());
}
