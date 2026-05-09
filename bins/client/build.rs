use tonic_prost_build::configure;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let include = protoc_bin_vendored::include_path()?;
    // SAFETY: build scripts are single-threaded
    unsafe {
        std::env::set_var("PROTOC", protoc);
        std::env::set_var("PROTOC_INCLUDE", include);
    }
    configure().compile_protos(&["proto/observer/v1/base_conn.proto"], &["proto"]).unwrap();
    Ok(())
}
