use std::{fs, path};

use image_atlas::*;

#[test]
fn usage() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 8,
        size: 2048,
        mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
        entries: &[AtlasEntry {
            texture: image::RgbImage::new(512, 512),
            mip: AtlasEntryMipOption::Single,
        }],
    })
    .unwrap();

    println!("{:?}", atlas.texcoords[0]);

    assert_eq!(atlas.texcoords.len(), 1);
    assert!(atlas.textures.len() <= 8);
    assert_eq!(atlas.textures[0].len(), 6);
}

#[test]
fn print() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 8,
        size: 2048,
        mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
        entries: &[
            AtlasEntry {
                texture: image::RgbImage::from_fn(512, 512, |_, _| image::Rgb([255, 0, 0])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(512, 256, |_, _| image::Rgb([0, 255, 0])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(32, 32, |_, _| image::Rgb([0, 0, 255])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([0, 255, 255])),
                mip: AtlasEntryMipOption::Single,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([255, 0, 255])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([255, 255, 0])),
                mip: AtlasEntryMipOption::Mirror,
            },
        ],
    })
    .unwrap();

    let dir_path = path::Path::new("target/img");
    if !dir_path.exists() {
        fs::create_dir("target/img").unwrap();
    }

    atlas
        .textures
        .into_vec()
        .into_iter()
        .enumerate()
        .for_each(|(i, texture)| {
            texture
                .into_vec()
                .into_iter()
                .enumerate()
                .for_each(|(j, mip_map)| {
                    let path = dir_path.join(format!("{}-{}.png", i, j));
                    mip_map.save(path).unwrap();
                });
        });
}
