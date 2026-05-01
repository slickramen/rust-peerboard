fn main() {
    let file_descriptor = protox::compile(
        ["proto/peerboard.proto"],
        ["proto/"],
    ).expect("failed to compile protos");

    prost_build::Config::new()
        .compile_fds(file_descriptor)
        .expect("failed to generate code");
}
