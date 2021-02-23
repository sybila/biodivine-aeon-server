use biodivine_aeon_server::{AeonSession, OPEN_MODEL};
mod test_main;
use std::sync::mpsc::channel;

fn main() {
    let mut sessions: Vec<AeonSession> = vec![AeonSession::new()];

    let (send, open) = channel();
    {
        *OPEN_MODEL.lock().unwrap() = Some(send);
    }
    loop {
        if let Ok(model) = open.try_recv() {
            println!("Model received. Starting session");
            sessions.push(AeonSession::new_with_model(&model));
        }

        let mut i = 0;
        while i < sessions.len() {
            let session = &mut sessions[i];
            if session.step().is_some() {
                i += 1;
            } else {
                sessions.remove(i);
            }
        }

        if sessions.is_empty() {
            break;
        }
    }
}
