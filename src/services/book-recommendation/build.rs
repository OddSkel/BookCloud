fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_prost_build::configure().compile_protos(
        &[
            "proto/common.proto",
            "proto/book_recommendation.proto"
            ],
        &[
            "proto",
            concat!(env!("CARGO_MANIFEST_DIR"), "/proto/"),
            ],
    )?;

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
