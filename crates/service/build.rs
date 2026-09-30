fn main() {
    // Icon and description shown in Task Manager and the Services console.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../gui/assets/dpimech.ico")
            .set("FileDescription", "DPIMech background service")
            .set("ProductName", "DPIMech");
        res.compile().expect("embedding Windows resources failed");
    }
}
