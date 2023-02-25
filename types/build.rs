fn main() {
    // Use this in build.rs
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .includes(&["src/proto"])
        .input("src/proto/sync.proto")
        .out_dir("src/proto")
        .run_from_script();
}
