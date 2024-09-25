use {
    std::{
        env,
        io,
    },
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    // Embed application icon
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon("icon/hide_desktop.ico")
            .compile()?;
    }
    Ok(())
}
