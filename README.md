# image-atlas

![Crates.io](https://img.shields.io/crates/v/image-atlas)
![Crates.io](https://img.shields.io/crates/l/image-atlas)

[**Documentation**](https://docs.rs/image-atlas)

This library provides a texture atlas generator for general purpose.

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
