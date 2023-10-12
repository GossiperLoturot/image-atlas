# image-atlas

![Crates.io](https://img.shields.io/crates/v/image-atlas)
![Crates.io](https://img.shields.io/crates/l/image-atlas)

[**Documentation**](https://docs.rs/image-atlas)

This library provides a texture atlas generator for general purpose. This library focuses on ease of use and simplicity.

There are multiple generation way

- No padding between texture elements
- With padding between texture elements
- With smart padding between texture elements for mip map generation.

and mip map generation option each texture elements

- Clamp
- Repeat
- Mirror

This library uses `image` crate for image backend and `rectangle-pack` crate for computing placements of atlas texture elements.

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

# License

This library is licensed under the [MIT license](LICENSE).
