use std::{fs, path};

use image::GenericImageView;
use image_atlas::*;

#[test]
fn usage() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
        entries: &[
            AtlasEntry {
                texture: image::RgbImage::new(512, 512),
                mip: AtlasEntryMipOption::Clamp,
            },
            AtlasEntry {
                texture: image::RgbImage::new(512, 256),
                mip: AtlasEntryMipOption::Clamp,
            },
        ],
    })
    .unwrap();

    assert_eq!(atlas.texcoords.len(), 2);
    println!("{:?}", atlas.texcoords[0]);
    println!("{:?}", atlas.texcoords[1]);

    assert_eq!(atlas.page_count, 1);
    assert_eq!(atlas.size, 2048);
    assert_eq!(atlas.mip_level_count, 6);
}

#[test]
#[ignore = "This test writes image files to file system."]
fn write_image() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
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
                mip: AtlasEntryMipOption::Clamp,
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

    assert_eq!(atlas.page_count, 1);

    let dir_path = path::Path::new("target/img");
    if !dir_path.exists() {
        fs::create_dir("target/img").unwrap();
    }

    atlas
        .textures
        .into_iter()
        .enumerate()
        .for_each(|(i, texture)| {
            texture
                .mip_maps
                .into_iter()
                .enumerate()
                .for_each(|(j, mip_map)| {
                    let path = dir_path.join(format!("{}-{}.png", i, j));
                    mip_map.save(path).unwrap();
                });
        });
}

#[test]
fn result_equality() {
    let entries = &[
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
            mip: AtlasEntryMipOption::Clamp,
        },
        AtlasEntry {
            texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([255, 0, 255])),
            mip: AtlasEntryMipOption::Repeat,
        },
        AtlasEntry {
            texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([255, 255, 0])),
            mip: AtlasEntryMipOption::Mirror,
        },
    ];

    let atlas0 = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::NoMip,
        entries,
    })
    .unwrap();

    let atlas1 = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::NoMipWithPadding(8),
        entries,
    })
    .unwrap();

    let atlas2 = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::Mip(AtlasMipFilter::Nearest),
        entries,
    })
    .unwrap();

    let atlas3 = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::MipWithPadding(AtlasMipFilter::Nearest, 8),
        entries,
    })
    .unwrap();

    let atlas4 = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 2048,
        mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
        entries,
    })
    .unwrap();

    fn extract_view<P: image::Pixel>(
        atlas: &Atlas<P>,
    ) -> Vec<image::SubImage<&image::ImageBuffer<P, Vec<P::Subpixel>>>> {
        atlas
            .texcoords
            .iter()
            .map(|texcoord| {
                let texture = &atlas.textures[texcoord.page as usize];
                let image = &texture.mip_maps[0];
                let x = texcoord.min_x;
                let y = texcoord.min_y;
                let w = texcoord.max_x - texcoord.min_x;
                let h = texcoord.max_y - texcoord.min_y;
                image.view(x, y, w, h)
            })
            .collect::<Vec<_>>()
    }

    let views0 = extract_view(&atlas0);
    let views1 = extract_view(&atlas1);
    let views2 = extract_view(&atlas2);
    let views3 = extract_view(&atlas3);
    let views4 = extract_view(&atlas4);

    for i in 0..entries.len() {
        assert!(views0[i].pixels().eq(views1[i].pixels()));
        assert!(views0[i].pixels().eq(views2[i].pixels()));
        assert!(views0[i].pixels().eq(views3[i].pixels()));
        assert!(views0[i].pixels().eq(views4[i].pixels()));
    }
}

#[test]
fn page_minimizing() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
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
                mip: AtlasEntryMipOption::Clamp,
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

    assert_eq!(atlas.page_count, 1);
}

#[test]
fn page_additional() {
    let atlas = create_atlas(&AtlasDescriptor {
        max_page_count: 2,
        size: 1024,
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
                texture: image::RgbImage::from_fn(512, 256, |_, _| image::Rgb([0, 255, 0])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(32, 32, |_, _| image::Rgb([0, 0, 255])),
                mip: AtlasEntryMipOption::Repeat,
            },
            AtlasEntry {
                texture: image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([0, 255, 255])),
                mip: AtlasEntryMipOption::Clamp,
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

    assert_eq!(atlas.page_count, 2);
}
