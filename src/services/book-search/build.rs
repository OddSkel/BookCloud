fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    // book-search, book-catalog, common — need server + client
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

    // author-catalog — client only, book-search doesn't implement this server
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../author-catalog/proto/author_catalog.proto"],
            &[
                "../author-catalog/proto/",
                "proto/", // needed for common.proto import
            ],
        )?;

    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=../author-catalog/proto");
    Ok(())
}