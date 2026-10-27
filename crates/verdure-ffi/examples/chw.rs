//! The whole SDK flow in one file, with no model on disk: start with a
//! licence, read the device, trust a signed manifest, judge a bundle, index
//! and search a guideline, and extract a form through the vendor's cloud
//! hook when no local engine is built. Run from the repo root:
//!
//!   cargo run -p verdure-ffi --example chw -- <licence-token>
//!
//! (Mint a token: `cargo run -p verdure-cli -- license issue --key ~/.verdure/issuer.key --vendor demo`)

use std::sync::Arc;

use verdure::{CloudFallback, DeleteScope, Doc, PolicyConfig, Task, Verdure, VerdureError};

struct DemoCloud;

impl CloudFallback for DemoCloud {
    fn transcribe(&self, _pcm_16k_mono: Vec<f32>) -> Result<String, VerdureError> {
        Ok(String::new())
    }
    fn extract(&self, _text: String, _schema_json: String) -> Result<String, VerdureError> {
        // A real vendor calls their own API here. This stands in for it.
        Ok(r#"{"client_name":"Amina","pregnant":true,"weeks_pregnant":28,"blood_pressure_systolic":150,"blood_pressure_diastolic":95,"danger_signs":["severe_headache"],"referred":true}"#.to_string())
    }
}

fn main() -> Result<(), VerdureError> {
    let token = std::env::args().nth(1).expect("pass a licence token");
    let root = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
    let read = |p: &str| std::fs::read_to_string(format!("{root}/{p}")).expect(p);

    let data_dir = std::env::temp_dir().join("verdure-chw-example");
    let sdk = Verdure::start(
        token,
        PolicyConfig::default(),
        data_dir.to_string_lossy().into_owned(),
    )?;
    let caps = sdk.capabilities();
    println!(
        "device: tier {} · {} MB available · engines built: {:?}",
        caps.tier, caps.available_ram_mb, caps.backends
    );
    println!("licence: {}", sdk.licence_summary());

    sdk.load_manifest(
        read("bundles/manifests/example.json"),
        read("bundles/manifests/example.json.sig"),
        read("crates/verdure-ffi/keys/issuer.pub"),
    )?;
    let fit = sdk.fit("chw-forms-P1".into())?;
    println!("chw-forms-P1 fits here: {} {:?}", fit.fits, fit.problems);
    match sdk.recommend(Task::Scribe) {
        Ok(b) => println!("recommended scribe bundle: {}", b.id),
        Err(e) => println!("no scribe bundle fits: {e}"),
    }

    let n = sdk.index(
        "guides".into(),
        vec![Doc {
            id: "anc".into(),
            title: "Antenatal care".into(),
            text: "Check blood pressure at every visit. Refer if above 140/90.".into(),
        }],
    )?;
    let hits = sdk.query(
        "guides".into(),
        "when do I refer for blood pressure?".into(),
        3,
    )?;
    println!(
        "indexed {n} chunk(s); top hit: {} — {}",
        hits[0].doc_id, hits[0].text
    );

    let text = "Client Amina, 28 weeks pregnant, BP 150 over 95, headache, referred.".to_string();
    let schema = read("bundles/manifests/schema-example-chw.json");
    match sdk.extract(text.clone(), schema.clone()) {
        Ok(out) => println!("extracted locally: {}", out.json),
        Err(e) => println!("no local engine: {e}"),
    }
    sdk.set_cloud_fallback(Some(Arc::new(DemoCloud)));
    let out = sdk.extract(text, schema)?;
    println!(
        "extracted via the vendor's cloud: {} {:?}",
        out.json, out.warnings
    );

    sdk.delete(DeleteScope::All)?;
    println!("deleted everything; done");
    Ok(())
}
