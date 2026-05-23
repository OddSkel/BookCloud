fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/common.proto",
                "proto/book_search.proto",
                "proto/book_catalog.proto",
            ],
            &[
                "proto/",
                concat!(env!("CARGO_MANIFEST_DIR"), "/proto/"),
            ],
        )?;
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["proto/author-catalog/author_catalog.proto"],
            &[
                "proto/author-catalog/",
                "proto/",
            ],
        )?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}