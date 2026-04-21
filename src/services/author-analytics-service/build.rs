fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe { std::env::set_var("PROTOC", protoc); }

    tonic_build::configure().compile_protos(
        &[
            "proto/common.proto",
            "proto/author_analytics.proto",
        ],
        &["proto"],
    )?;

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}