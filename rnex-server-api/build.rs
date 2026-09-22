use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../rnex-server-api-grpc/");

    let protos: Vec<String> = fs::read_dir("../rnex-server-api-grpc/")?
        .filter(|e| {
            e.as_ref().is_ok_and(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.ends_with(".proto"))
            })
        })
        .filter_map(|e| {
            e.ok()
                .map(|v| v.path().to_str().map(|v| v.to_owned()))
                .flatten()
        })
        .collect();
    tonic_prost_build::configure()
        .compile_protos(&protos[..], &["../rnex-server-api-grpc/".to_owned()])?;
    Ok(())
}
