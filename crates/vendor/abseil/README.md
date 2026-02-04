# Abseil Rust

Rust port of Google's [Abseil Common Libraries (C++)](https://github.com/abseil/abseil-cpp).

## About Abseil

Abseil is an open-source collection of C++ library code designed to augment the C++ standard library. The Abseil code base is collected from Google's own internal code base.

This Rust port aims to provide the same functionality and API patterns as the original C++ version, adapted to Rust idioms where appropriate.

## Status

This is an independent community project, not an official Google product.

## License

Copyright 2025 The Abseil Authors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## Original Abseil

This is a Rust port of the C++ Abseil library. For the official C++ version, see:

- GitHub: https://github.com/abseil/abseil-cpp
- Documentation: https://abseil.io/

## Modules

Currently implemented:

- `absl_base` - Base utilities (call_once, attributes, optimization, macros)
- `absl_log` - Logging utilities (LOG, CHECK, VLOG, severity)

Planned:

- `absl_strings` - String utilities (string_view, str_split)
- `absl_container` - Container utilities (flat_hash_map)
- `absl_time` - Time utilities

## Usage

```toml
[dependencies]
abseil = "0.1"
```

```rust
use abseil::prelude::*;

fn main() {
    // Call once
    static INIT: OnceFlag = OnceFlag::new();
    call_once(&INIT, || {
        println!("Initialized!");
    });

    // Logging
    LOG!(INFO, "Hello, Abseil!");
}
```

## Development Status

This project is in early development. The API may change significantly.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
