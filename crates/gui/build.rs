fn main() {
    // Translations: lang/<lang>/LC_MESSAGES/dpimech-gui.po, shared with Rust-side text
    // (src/i18n.rs), so no per-component context. Update them with tools/i18n.py.
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    println!("cargo:rerun-if-changed=lang");
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint build failed");

    // Icon and version info shown by Explorer, the taskbar and Task Manager.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/dpimech.ico")
            .set("FileDescription", "DPIMech")
            .set("ProductName", "DPIMech");
        res.compile().expect("embedding Windows resources failed");
    }
}
