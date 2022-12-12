#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("ProductName", "GuiNistRs");
    res.set("FileDescription", "GuiNistRs");
    res.set("LegalCopyright", "Copyright (C) 2022");
    res.set_icon("icon/guinist.ico");
    res.compile()
        .expect("Failed to run the Windows resource compiler (rc.exe)");
}

#[cfg(not(windows))]
fn main() {}
