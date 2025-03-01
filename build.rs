fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file to generate Rust code
    tonic_build::compile_protos("proto/chat.proto")?;
    Ok(())
}
