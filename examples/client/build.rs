fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use the proto file from the parent project
    tonic_build::compile_protos("../../proto/chat.proto")?;
    Ok(())
}
