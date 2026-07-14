use tonic_prost_build::configure;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let include = protoc_bin_vendored::include_path()?;
    // SAFETY: build scripts are single-threaded
    unsafe {
        std::env::set_var("PROTOC", protoc);
        std::env::set_var("PROTOC_INCLUDE", include);
    }
    configure()
        .compile_protos(
            &[
                "proto/observer/v1/base_conn.proto",
                "proto/observer/v1/metrics/cpu.proto",
                "proto/observer/v1/metrics/memory.proto",
                "proto/observer/v1/metrics/disk.proto",
                "proto/observer/v1/metrics/network.proto",
                "proto/observer/v1/metrics/system.proto",
                "proto/observer/v1/metrics/process.proto",
                "proto/observer/v1/metrics/container_runtime.proto",
                "proto/observer/v1/metrics/speedtest.proto",
            ],
            &["proto"],
        )
        .unwrap();
    Ok(())
}
