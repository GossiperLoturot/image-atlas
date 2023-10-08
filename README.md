# image-atlas

![Crates.io](https://img.shields.io/crates/v/image-atlas)
![Crates.io](https://img.shields.io/crates/l/image-atlas)

[**Documentation**](https://docs.rs/image-atlas)

This library provides a texture atlas generator for general purpose. This library focuses on ease of use and simplicity.

There are multiple generation way

- No gaps between texture elements
- Simple gap between texture elements
- Smart gap between texture elements for mip map generation.

and mip map generation option each texture elements

- Single
- Repeat

This library uses `image` crate for image backend and `rectangle-pack` crate for computing placements of atlas texture elements.

# Examples

```rust
use std::collections::hash_map::RandomState;
use image_atlas::*;

let atlas = create_atlas::<_, _, RandomState>(&AtlasDescriptor {
    max_page_count: 8,
    size: 2048,
    mip: AtlasMipOption::Block(32),
    entries: &[AtlasEntry {
        key: "example1",
        texture: image::RgbImage::new(512, 512),
        mip: AtlasEntryMipOption::Single,
    }],
})
.unwrap();

println!("{:?}", atlas.texcoords.get("example1"));
```

# Installation

```shell
cargo add image image-atlas
```

# License

This library is licensed under the [MIT license](LICENSE).
