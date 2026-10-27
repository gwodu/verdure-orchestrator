//! Smallest possible use of the SDK: start with the sandbox licence and print
//! what this device is. Everything else is in docs/sdk-integration-guide.md.
use verdure::{PolicyConfig, Verdure};

fn main() {
    let token = std::fs::read_to_string("../../bundles/sandbox.token")
        .expect("run from inside sandbox/<project>/")
        .trim()
        .to_string();
    let data_dir = std::env::temp_dir().join("verdure-sandbox");
    let sdk = Verdure::start(token, PolicyConfig::default(), data_dir.to_string_lossy().into_owned())
        .expect("the sandbox licence is valid for the built-in key");
    let caps = sdk.capabilities();
    println!("tier {} · {} MB of RAM free now · {} MB disk free · engines built: {:?}", caps.tier, caps.available_ram_mb, caps.free_disk_mb, caps.backends);
    for note in caps.notes {
        println!("note: {note}");
    }
}
