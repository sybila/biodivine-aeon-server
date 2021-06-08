use std::io::Read;
use biodivine_lib_param_bn::BooleanNetwork;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    let model = BooleanNetwork::try_from_bnet(buffer.as_str()).unwrap();
    println!("{}", model.to_string());
}