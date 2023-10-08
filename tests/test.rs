use std::collections::hash_map::RandomState;

use image_atlas::*;

#[test]
fn usage() {
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

    assert_eq!(atlas.texcoords.len(), 1);
    assert!(atlas.textures.len() <= 8);
    assert_eq!(atlas.textures[0].len(), 6);
}
