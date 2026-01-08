use std::process::Command;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    let my_path = args[0].clone();
    let server_path = format!(
        "{}/biodivine-aeon-server",
        my_path.strip_suffix("launcher").unwrap()
    );

    println!("Launching biodivine-aeon-server from {}", server_path);

    let out = Command::new("open")
        .args(["-a", "Terminal", server_path.as_str()])
        .output()
        .unwrap();

    println!("{:?}", out.status);

    println!("Done");
}
