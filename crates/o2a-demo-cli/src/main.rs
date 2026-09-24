use std::env;

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "help".to_owned());
    match command.as_str() {
        "create-identity" | "rotate-controller" | "issue-attestation" | "verify-package" => {
            eprintln!("{command}: scaffolded but not implemented in Item 1");
            std::process::exit(2);
        }
        _ => {
            println!(
                "o2a-demo commands: create-identity, rotate-controller, \
                 issue-attestation, verify-package"
            );
            println!("{}", o2a_demo_core::evaluation_boundary());
            println!("RGB runtime: {}", o2a_demo_rgb::RGB_RUNTIME_REV);
        }
    }
}
