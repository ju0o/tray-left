# Windows third-party notices

This inventory is generated from the locked Windows MSVC Cargo graph on 2026-10-28. It includes packages in `cargo tree --locked --target x86_64-pc-windows-msvc --edges normal,build`; procedural-macro and build-time packages are included even when they do not become runtime DLLs. The table describes Cargo dependency reachability, not a DLL inventory.

Jusage's own code is offered under the MIT License in [`LICENSE`](LICENSE), Copyright (c) 2026 ju0o. That license is separate from the dependency licenses below. Each upstream license, copying, or notice file is copied byte-for-byte under `THIRD_PARTY_LICENSES/`; upstream names and copyright notices are retained. No dependency license text is replaced with Jusage's copyright.

The resolved graph has 48 package/version entries: 45 in the normal-edge graph and 3 additional build-only entries. Cargo metadata declares MIT, Apache-2.0, Unicode-3.0, and Unlicense expressions in this graph; no GPL-only expression appeared. Windows system APIs, the Windows SDK, and the MSVC runtime are supplied by the platform/toolchain and are not bundled as Cargo crates here.

| Package | Version | Cargo license expression | Graph | Preserved upstream files |
| --- | --- | --- | --- | --- |
| `base64` | `0.22.1` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/base64-0.22.1/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/base64-0.22.1/LICENSE-MIT` |
| `displaydoc` | `0.2.7` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/displaydoc-0.2.7/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/displaydoc-0.2.7/LICENSE-MIT` |
| `form_urlencoded` | `1.2.2` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/form_urlencoded-1.2.2/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/form_urlencoded-1.2.2/LICENSE-MIT` |
| `icu_collections` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_collections-2.4.0/LICENSE` |
| `icu_locale_core` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_locale_core-2.4.0/LICENSE` |
| `icu_normalizer` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_normalizer-2.4.0/LICENSE` |
| `icu_normalizer_data` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_normalizer_data-2.4.0/LICENSE` |
| `icu_properties` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_properties-2.4.0/LICENSE` |
| `icu_properties_data` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_properties_data-2.4.0/LICENSE` |
| `icu_provider` | `2.4.0` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/icu_provider-2.4.0/LICENSE` |
| `idna` | `1.1.0` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/idna-1.1.0/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/idna-1.1.0/LICENSE-MIT` |
| `idna_adapter` | `1.2.2` | `Apache-2.0 OR MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/idna_adapter-1.2.2/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/idna_adapter-1.2.2/LICENSE-MIT` |
| `itoa` | `1.0.18` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/itoa-1.0.18/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/itoa-1.0.18/LICENSE-MIT` |
| `litemap` | `0.8.3` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/litemap-0.8.3/LICENSE` |
| `log` | `0.4.34` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/log-0.4.34/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/log-0.4.34/LICENSE-MIT` |
| `memchr` | `2.8.3` | `Unlicense OR MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/memchr-2.8.3/COPYING`, `THIRD_PARTY_LICENSES/memchr-2.8.3/LICENSE-MIT`, `THIRD_PARTY_LICENSES/memchr-2.8.3/UNLICENSE` |
| `native-tls` | `0.2.18` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/native-tls-0.2.18/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/native-tls-0.2.18/LICENSE-MIT` |
| `once_cell` | `1.21.4` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/once_cell-1.21.4/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/once_cell-1.21.4/LICENSE-MIT` |
| `percent-encoding` | `2.3.2` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/percent-encoding-2.3.2/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/percent-encoding-2.3.2/LICENSE-MIT` |
| `potential_utf` | `0.1.6` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/potential_utf-0.1.6/LICENSE` |
| `proc-macro2` | `1.0.107` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/proc-macro2-1.0.107/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/proc-macro2-1.0.107/LICENSE-MIT` |
| `quote` | `1.0.47` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/quote-1.0.47/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/quote-1.0.47/LICENSE-MIT` |
| `schannel` | `0.1.30` | `MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/schannel-0.1.30/LICENSE.md` |
| `serde` | `1.0.229` | `MIT OR Apache-2.0` | build-only | `THIRD_PARTY_LICENSES/serde-1.0.229/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/serde-1.0.229/LICENSE-MIT` |
| `serde_core` | `1.0.229` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/serde_core-1.0.229/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/serde_core-1.0.229/LICENSE-MIT` |
| `serde_json` | `1.0.154` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/serde_json-1.0.154/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/serde_json-1.0.154/LICENSE-MIT` |
| `smallvec` | `1.16.4` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/smallvec-1.16.4/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/smallvec-1.16.4/LICENSE-MIT` |
| `stable_deref_trait` | `1.2.1` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/stable_deref_trait-1.2.1/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/stable_deref_trait-1.2.1/LICENSE-MIT` |
| `syn` | `3.0.9` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/syn-3.0.9/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/syn-3.0.9/LICENSE-MIT` |
| `synstructure` | `0.14.0` | `MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/synstructure-0.14.0/LICENSE` |
| `tinystr` | `0.8.5` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/tinystr-0.8.5/LICENSE` |
| `toml` | `0.5.11` | `MIT/Apache-2.0` | build-only | `THIRD_PARTY_LICENSES/toml-0.5.11/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/toml-0.5.11/LICENSE-MIT` |
| `unicode-ident` | `1.0.27` | `(MIT OR Apache-2.0) AND Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/unicode-ident-1.0.27/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/unicode-ident-1.0.27/LICENSE-MIT`, `THIRD_PARTY_LICENSES/unicode-ident-1.0.27/LICENSE-UNICODE` |
| `ureq` | `2.12.1` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/ureq-2.12.1/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/ureq-2.12.1/LICENSE-MIT` |
| `url` | `2.5.8` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/url-2.5.8/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/url-2.5.8/LICENSE-MIT` |
| `utf8_iter` | `1.0.4` | `Apache-2.0 OR MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/utf8_iter-1.0.4/LICENSE-APACHE`, `THIRD_PARTY_LICENSES/utf8_iter-1.0.4/LICENSE-MIT` |
| `windows-link` | `0.2.1` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/windows-link-0.2.1/license-apache-2.0`, `THIRD_PARTY_LICENSES/windows-link-0.2.1/license-mit` |
| `windows-sys` | `0.61.2` | `MIT OR Apache-2.0` | normal-edge graph | `THIRD_PARTY_LICENSES/windows-sys-0.61.2/license-apache-2.0`, `THIRD_PARTY_LICENSES/windows-sys-0.61.2/license-mit` |
| `winres` | `0.1.12` | `MIT` | build-only | `THIRD_PARTY_LICENSES/winres-0.1.12/LICENSE` |
| `writeable` | `0.6.4` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/writeable-0.6.4/LICENSE` |
| `yoke` | `0.8.4` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/yoke-0.8.4/LICENSE` |
| `yoke-derive` | `0.8.4` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/yoke-derive-0.8.4/LICENSE` |
| `zerofrom` | `0.1.8` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/zerofrom-0.1.8/LICENSE` |
| `zerofrom-derive` | `0.1.8` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/zerofrom-derive-0.1.8/LICENSE` |
| `zerotrie` | `0.2.5` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/zerotrie-0.2.5/LICENSE` |
| `zerovec` | `0.11.9` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/zerovec-0.11.9/LICENSE` |
| `zerovec-derive` | `0.11.7` | `Unicode-3.0` | normal-edge graph | `THIRD_PARTY_LICENSES/zerovec-derive-0.11.7/LICENSE` |
| `zmij` | `1.0.23` | `MIT` | normal-edge graph | `THIRD_PARTY_LICENSES/zmij-1.0.23/LICENSE-MIT` |
