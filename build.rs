fn main() {
    println!("cargo:rerun-if-changed=assets/windows/soundora.rc");
    println!("cargo:rerun-if-changed=assets/icons/soundora.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // GPUI 0.2.2 loads Windows window icons from resource ID 1 in the executable.
        embed_resource::compile_for(
            "assets/windows/soundora.rc",
            ["audio-player"],
            embed_resource::NONE,
        )
        .manifest_required()
        .expect("failed to embed the Soundora Windows icon");
    }
}
