use std::env;

fn main() {
    let wdm  = env::var_os("CARGO_FEATURE_DRIVER_WDM").is_some();
    let kmdf = env::var_os("CARGO_FEATURE_DRIVER_KMDF").is_some();
    let umdf = env::var_os("CARGO_FEATURE_DRIVER_UMDF").is_some();

    // Default to WDM
    let model = if umdf { "UMDF" } else if kmdf { "KMDF" } else { "WDM" };

    // Define the cfg
    println!(r#"cargo:rustc-cfg=driver_model__driver_type="{model}""#);

    // Silence/check the unexpected_cfgs lint
    println!(r#"cargo:rustc-check-cfg=cfg(driver_model__driver_type, values("WDM","KMDF","UMDF"))"#);
}