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

#[test]
fn print() {
    let atlas = create_atlas::<_, _, RandomState>(&AtlasDescriptor {
        max_page_count: 8,
        size: 2048,
        mip: AtlasMipOption::Block(32),
        entries: &[
            AtlasEntry {
                key: "example1",
                texture: image::RgbImage::from_fn(512, 512, |x, y| image::Rgb([255, 0, 0])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example2",
                texture: image::RgbImage::from_fn(512, 256, |x, y| image::Rgb([0, 255, 0])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example3",
                texture: image::RgbImage::from_fn(32, 32, |x, y| image::Rgb([0, 0, 255])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example4",
                texture: image::RgbImage::from_fn(256, 256, |x, y| image::Rgb([0, 255, 255])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example5",
                texture: image::RgbImage::from_fn(4, 4, |x, y| image::Rgb([255, 0, 255])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example6",
                texture: image::RgbImage::from_fn(3, 5, |x, y| image::Rgb([255, 255, 0])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                key: "example7",
                texture: image::RgbImage::from_fn(3, 5, |x, y| image::Rgb([255, 255, 255])),
                mip: AtlasEntryMipOption::Repeat,
            },
        ],
    })
    .unwrap();

    // std::fs::create_dir("img").unwrap();
    // atlas
    //     .textures
    //     .into_vec()
    //     .into_iter()
    //     .enumerate()
    //     .for_each(|(i, texture)| {
    //         texture
    //             .into_vec()
    //             .into_iter()
    //             .enumerate()
    //             .for_each(|(j, mip_map)| {
    //                 let path = format!("img/{}-{}.png", i, j);
    //                 mip_map.save(path).unwrap();
    //             });
    //     });
}
