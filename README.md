# image-atlas

[![crates.io](https://img.shields.io/crates/v/image-atlas)](https://crates.io/crates/image-atlas)
[![doc.rs](https://img.shields.io/docsrs/image-atlas)](https://docs.rs/image-atlas)

[**Documentation**](https://docs.rs/image-atlas)

This library provides a general-purpose atlas texture generator with a focus on ease of use and simplicity.

There are multiple generation methods and mip map options.

- No padding between elements
- With padding between elements
- With smart padding between elements for mip map generation.

This library uses `image` crate for image processing and `rectangle-pack` crate for computing element layout.

# Examples

```rust
use image_atlas::*;

let atlas = create_atlas(&AtlasDescriptor {
    max_page_count: 8,
    size: 2048,
    mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
    entries: &[AtlasEntry {
        texture: image::RgbImage::new(512, 512),
        mip: AtlasEntryMipOption::Clamp,
    }],
})
.unwrap();

println!("{:?}", atlas.texcoords[0]);
```

# Installation

```shell
cargo add image image-atlas
```
