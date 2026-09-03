fn main() -> Result<(), Box<dyn std::error::Error>> {
    cynic_codegen::register_schema("wikijs")
        .from_sdl_file("schemas/wikijs.graphql")?
        .as_default()?;

    Ok(())
}
