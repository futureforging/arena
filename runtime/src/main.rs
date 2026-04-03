cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        mod plugins;

        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_keyvalue::{KeyValueDefault, WasiKeyValue};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_vault::WasiVault;
        use plugins::VaultAnthropicLocalFile;

        omnia::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiKeyValue: KeyValueDefault,
                WasiOtel: OtelDefault,
                WasiVault: VaultAnthropicLocalFile,
            }
        });
    } else {
        fn main() {}
    }
}
