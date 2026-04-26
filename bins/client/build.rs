fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let include = protoc_bin_vendored::include_path()?;
    // SAFETY: build scripts are single-threaded
    unsafe {
        std::env::set_var("PROTOC", protoc);
        std::env::set_var("PROTOC_INCLUDE", include);
    }
    tonic_prost_build::compile_protos("proto/metrics.proto")?;
    Ok(())
}
