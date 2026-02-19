~/Documents/ClipWallet/
├── Cargo.toml
├── install.sh                        ← Phase 6
├── build_release.sh                  ← Phase 6
├── assets/
│   └── com.clipwallet.agent.plist.template
├── dist/                             ← Phase 6 (generated)
│   └── clipwallet-0.1.0-*.zip
└── src/
    ├── main.rs                       ← Phase 6 final
    ├── engine.rs                     ← Phase 5 final
    ├── hotkey/
    │   ├── mod.rs
    │   ├── chord.rs                  ← Phase 3
    │   └── injector.rs               ← Phase 2
    ├── storage/
    │   ├── mod.rs
    │   ├── ram.rs                    ← Phase 3
    │   ├── disk.rs                   ← Phase 4 + 6
    │   └── encrypted.rs              ← Phase 4
    ├── clipboard/
    │   ├── mod.rs
    │   ├── types.rs                  ← Phase 5
    │   ├── mime.rs                   ← Phase 5
    │   └── pasteboard.rs             ← Phase 5
    └── daemon/
        ├── mod.rs                    ← Phase 6
        ├── plist.rs                  ← Phase 6
        └── status.rs                 ← Phase 6