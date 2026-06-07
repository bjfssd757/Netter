use std::path::PathBuf;

fn main() -> Result<(), std::io::Error> {
    // let compile_client = std::env::var("CARGO_FEATURE_CLIENT").is_ok();
    // let compile_server = std::env::var("CARGO_FEATURE_SERVER").is_ok();

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = manifest_dir.parent().unwrap().join("proto");

    println!("cargo:rerun-if-changed={}", proto_dir.display());

    let proto_files = [
        proto_dir.join("supervisor.proto"),
        proto_dir.join("cli.proto"),
    ];
    let shared_proto = proto_dir.join("shared.proto");

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(false)
        .compile_protos(&[shared_proto], &[proto_dir.clone()])?;

    tonic_prost_build::configure()
        // .build_client(compile_client)
        // .build_server(compile_server)
        .extern_path(".netter.shared.v1", "crate::proto_shared::v1")
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &proto_files,
            &[proto_dir]
        )?;

    Ok(())
}