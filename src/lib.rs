//! # image-atlas
//!
//! This library provides a texture atlas generator for general purpose. This library focuses on ease of use and simplicity.
//!
//! There are multiple generation way
//!
//! - No padding between texture elements
//! - With padding between texture elements
//! - With smart padding between texture elements for mip map generation.
//!
//! and mip map generation option each texture elements
//!
//! - Clamp
//! - Repeat
//! - Mirror
//!
//! This library uses `image` crate for image backend and `rectangle-pack` crate for computing placements of atlas texture elements.
//!
//! # Examples
//!
//! ```rust
//! use image_atlas::*;
//!
//! let atlas = create_atlas(&AtlasDescriptor {
//!     max_page_count: 8,
//!     size: 2048,
//!     mip: AtlasMipOption::MipWithBlock(AtlasMipFilter::Lanczos3, 32),
//!     entries: &[AtlasEntry {
//!         texture: image::RgbImage::new(512, 512),
//!         mip: AtlasEntryMipOption::Clamp,
//!     }],
//! })
//! .unwrap();
//!
//! println!("{:?}", atlas.texcoords[0]);
//! ```

use std::{collections::BTreeMap, error, fmt, ops};

/// A mip map filter for texture atlas
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtlasMipFilter {
    #[default]
    Nearest,
    Linear,
    Cubic,
    Gaussian,
    Lanczos3,
}

impl From<AtlasMipFilter> for image::imageops::FilterType {
    #[inline]
    fn from(value: AtlasMipFilter) -> Self {
        match value {
            AtlasMipFilter::Nearest => Self::Nearest,
            AtlasMipFilter::Linear => Self::Triangle,
            AtlasMipFilter::Cubic => Self::CatmullRom,
            AtlasMipFilter::Gaussian => Self::Gaussian,
            AtlasMipFilter::Lanczos3 => Self::Lanczos3,
        }
    }
}

/// A mip map generation method for texture atlas
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtlasMipOption {
    #[default]
    NoMip,
    NoMipWithPadding(u32),
    Mip(AtlasMipFilter),
    MipWithPadding(AtlasMipFilter, u32),
    MipWithBlock(AtlasMipFilter, u32),
}

/// A mip map generation method each texture elements
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtlasEntryMipOption {
    #[default]
    Clamp,
    Repeat,
    Mirror,
}

/// A texture element description
#[derive(Clone, PartialEq, Eq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtlasEntry<I: image::GenericImageView> {
    pub texture: I,
    pub mip: AtlasEntryMipOption,
}

/// A texture atlas description
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct AtlasDescriptor<'a, I: image::GenericImageView> {
    pub max_page_count: u32,
    pub size: u32,
    pub mip: AtlasMipOption,
    pub entries: &'a [AtlasEntry<I>],
}

/// Creates a new texture atlas.
#[rustfmt::skip]
pub fn create_atlas<I>(desc: &AtlasDescriptor<'_, I>) -> Result<Atlas<I::Pixel>, AtlasError>
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    match desc.mip {
        AtlasMipOption::NoMip => {
            create_atlas_with_padding(desc.max_page_count, desc.size, 0, desc.entries)
        }
        AtlasMipOption::NoMipWithPadding(padding) => {
            create_atlas_with_padding(desc.max_page_count, desc.size, padding, desc.entries)
        }
        AtlasMipOption::Mip(filter) => {
            create_atlas_mip_with_padding(desc.max_page_count, desc.size, filter, 0, desc.entries)
        }
        AtlasMipOption::MipWithPadding(filter, padding) => {
            create_atlas_mip_with_padding(desc.max_page_count, desc.size, filter, padding, desc.entries)
        }
        AtlasMipOption::MipWithBlock(filter, block_size) => {
            create_atlas_mip_with_block(desc.max_page_count, desc.size, filter, block_size, desc.entries)
        }
    }
}

#[inline]
fn create_atlas_with_padding<I>(
    max_page_count: u32,
    size: u32,
    padding: u32,
    entries: &[AtlasEntry<I>],
) -> Result<Atlas<I::Pixel>, AtlasError>
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    if max_page_count == 0 {
        return Err(AtlasError::ZeroMaxPageCount);
    }

    if entries.is_empty() {
        return Err(AtlasError::ZeroEntry);
    }

    let mut rects = rectangle_pack::GroupedRectsToPlace::<_, ()>::new();
    for (i, entry) in entries.iter().enumerate() {
        let rect = rectangle_pack::RectToInsert::new(
            entry.texture.width() + padding * 2,
            entry.texture.height() + padding * 2,
            1,
        );
        rects.push_rect(i, None, rect);
    }

    let mut target_bins = BTreeMap::new();
    target_bins.insert(
        (),
        rectangle_pack::TargetBin::new(size, size, max_page_count),
    );

    let locations = rectangle_pack::pack_rects(
        &rects,
        &mut target_bins,
        &rectangle_pack::volume_heuristic,
        &rectangle_pack::contains_smallest_box,
    )?;

    let mut page_count = 0;
    let mut texcoords = vec![Texcoord::default(); entries.len()];
    for (&i, (_, location)) in locations.packed_locations() {
        page_count = u32::max(page_count, location.z() + 1);

        let texcoord = Texcoord {
            page: location.z(),
            min_x: location.x() + padding,
            min_y: location.y() + padding,
            max_x: location.x() + location.width() - padding,
            max_y: location.y() + location.height() - padding,
            size,
        };
        texcoords[i] = texcoord;
    }

    let mut textures = Textures::new_with(page_count, size, 1);
    for (&i, (_, location)) in locations.packed_locations() {
        let entry = &entries[i];

        let src = resample(
            &entry.texture,
            entry.mip,
            padding,
            padding,
            location.width(),
            location.height(),
        );

        let target = &mut textures[location.z() as usize][0];
        image::imageops::replace(target, &src, location.x() as i64, location.y() as i64);
    }

    Ok(Atlas {
        textures,
        texcoords,
    })
}

#[inline]
fn create_atlas_mip_with_padding<I>(
    max_page_count: u32,
    size: u32,
    filter: AtlasMipFilter,
    padding: u32,
    entries: &[AtlasEntry<I>],
) -> Result<Atlas<I::Pixel>, AtlasError>
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    if max_page_count == 0 {
        return Err(AtlasError::ZeroMaxPageCount);
    }

    if !size.is_power_of_two() {
        return Err(AtlasError::InvalidSize(size));
    }

    if entries.is_empty() {
        return Err(AtlasError::ZeroEntry);
    }

    let mut rects = rectangle_pack::GroupedRectsToPlace::<_, ()>::new();
    for (i, entry) in entries.iter().enumerate() {
        let rect = rectangle_pack::RectToInsert::new(
            entry.texture.width() + padding * 2,
            entry.texture.height() + padding * 2,
            1,
        );
        rects.push_rect(i, None, rect);
    }

    let mut target_bins = BTreeMap::new();
    target_bins.insert(
        (),
        rectangle_pack::TargetBin::new(size, size, max_page_count),
    );

    let locations = rectangle_pack::pack_rects(
        &rects,
        &mut target_bins,
        &rectangle_pack::volume_heuristic,
        &rectangle_pack::contains_smallest_box,
    )?;

    let mut page_count = 0;
    let mut texcoords = vec![Texcoord::default(); entries.len()];
    for (&i, (_, location)) in locations.packed_locations() {
        page_count = u32::max(page_count, location.z() + 1);

        let texcoord = Texcoord {
            page: location.z(),
            min_x: location.x() + padding,
            min_y: location.y() + padding,
            max_x: location.x() + location.width() - padding,
            max_y: location.y() + location.height() - padding,
            size,
        };
        texcoords[i] = texcoord;
    }

    let mip_level_count = size.ilog2() + 1;
    let mut textures = Textures::new_with(page_count, size, mip_level_count);
    for (&i, (_, location)) in locations.packed_locations() {
        let entry = &entries[i];

        let src = resample(
            &entry.texture,
            entry.mip,
            padding,
            padding,
            location.width(),
            location.height(),
        );

        let target = &mut textures[location.z() as usize][0];
        image::imageops::replace(target, &src, location.x() as i64, location.y() as i64);
    }

    for mip_level in 1..mip_level_count {
        let size = size >> mip_level;

        for page in 0..page_count {
            let src = &textures[page as usize][0];

            let mip_map = image::imageops::resize(src, size, size, filter.into());

            let target = &mut textures[page as usize][mip_level as usize];
            image::imageops::replace(target, &mip_map, 0, 0);
        }
    }

    Ok(Atlas {
        textures,
        texcoords,
    })
}

#[inline]
fn create_atlas_mip_with_block<I>(
    max_page_count: u32,
    size: u32,
    filter: AtlasMipFilter,
    block_size: u32,
    entries: &[AtlasEntry<I>],
) -> Result<Atlas<I::Pixel>, AtlasError>
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    if max_page_count == 0 {
        return Err(AtlasError::ZeroMaxPageCount);
    }

    if !size.is_power_of_two() {
        return Err(AtlasError::InvalidSize(size));
    }

    if !block_size.is_power_of_two() {
        return Err(AtlasError::InvalidBlockSize(block_size));
    }

    if entries.is_empty() {
        return Err(AtlasError::ZeroEntry);
    }

    let padding = block_size >> 1;

    let mut rects = rectangle_pack::GroupedRectsToPlace::<_, ()>::new();
    for (i, entry) in entries.iter().enumerate() {
        let rect = rectangle_pack::RectToInsert::new(
            ((entry.texture.width() + block_size) as f32 / block_size as f32).ceil() as u32,
            ((entry.texture.height() + block_size) as f32 / block_size as f32).ceil() as u32,
            1,
        );
        rects.push_rect(i, None, rect);
    }

    let bin_size = size / block_size;
    let mut target_bins = BTreeMap::new();
    target_bins.insert(
        (),
        rectangle_pack::TargetBin::new(bin_size, bin_size, max_page_count),
    );

    let locations = rectangle_pack::pack_rects(
        &rects,
        &mut target_bins,
        &rectangle_pack::volume_heuristic,
        &rectangle_pack::contains_smallest_box,
    )?;

    let mut page_count = 0;
    let mut texcoords = vec![Texcoord::default(); entries.len()];
    for (&i, (_, location)) in locations.packed_locations() {
        page_count = u32::max(page_count, location.z() + 1);

        let texcoord = Texcoord {
            page: location.z(),
            min_x: location.x() * block_size + padding,
            min_y: location.y() * block_size + padding,
            max_x: (location.x() + location.width()) * block_size - padding,
            max_y: (location.y() + location.height()) * block_size - padding,
            size,
        };
        texcoords[i] = texcoord;
    }

    let mip_level_count = block_size.ilog2() + 1;
    let mut textures = Textures::new_with(page_count, size, mip_level_count);
    for (&i, (_, location)) in locations.packed_locations() {
        let entry = &entries[i];

        let src = resample(
            &entry.texture,
            entry.mip,
            padding,
            padding,
            location.width() * block_size,
            location.height() * block_size,
        );

        for mip_level in 0..mip_level_count {
            let width = src.width() >> mip_level;
            let height = src.height() >> mip_level;
            let mip_map = image::imageops::resize(&src, width, height, filter.into());

            let target = &mut textures[location.z() as usize][mip_level as usize];
            let x = location.x() as i64 * (block_size >> mip_level) as i64;
            let y = location.y() as i64 * (block_size >> mip_level) as i64;
            image::imageops::replace(target, &mip_map, x, y);
        }
    }

    Ok(Atlas {
        textures,
        texcoords,
    })
}

#[inline]
#[rustfmt::skip]
fn resample<I>(
    src: &I,
    mip: AtlasEntryMipOption,
    shift_x: u32,
    shift_y: u32,
    width: u32,
    height: u32,
) -> image::ImageBuffer<I::Pixel, Vec<<I::Pixel as image::Pixel>::Subpixel>>
where
    I: image::GenericImage,
{
    let mut target = image::ImageBuffer::new(width, height);
    match mip {
        AtlasEntryMipOption::Clamp => {
            for x in 0..width {
                for y in 0..height {
                    let sx = (x as i32 - shift_x as i32).max(0).min(src.width() as i32 - 1);
                    let sy = (y as i32 - shift_y as i32).max(0).min(src.height() as i32 - 1);
                    *target.get_pixel_mut(x, y) = src.get_pixel(sx as u32, sy as u32);
                }
            }
        }
        AtlasEntryMipOption::Repeat => {
            for x in 0..width {
                for y in 0..height {
                    let sx = (x as i32 - shift_x as i32).rem_euclid(src.width() as i32);
                    let sy = (y as i32 - shift_y as i32).rem_euclid(src.height() as i32);
                    *target.get_pixel_mut(x, y) = src.get_pixel(sx as u32, sy as u32);
                }
            }
        }
        AtlasEntryMipOption::Mirror => {
            for x in 0..width {
                for y in 0..height {
                    let xx = (x as i32 - shift_x as i32).div_euclid(src.width() as i32);
                    let yy = (y as i32 - shift_y as i32).div_euclid(src.height() as i32);
                    let mut sx = (x as i32 - shift_x as i32).rem_euclid(src.width() as i32);
                    let mut sy = (y as i32 - shift_y as i32).rem_euclid(src.height() as i32);
                    if xx & 1 == 0 { sx = src.width() as i32 - 1 - sx; }
                    if yy & 1 == 0 { sy = src.height() as i32 - 1 - sy; }
                    *target.get_pixel_mut(x, y) = src.get_pixel(sx as u32, sy as u32);
                }
            }
        }
    }
    target
}

/// A texture atlas
#[derive(Clone, Default)]
pub struct Atlas<P: image::Pixel> {
    pub textures: Textures<P>,
    pub texcoords: Vec<Texcoord>,
}

impl<P> fmt::Debug for Atlas<P>
where
    P: image::Pixel + fmt::Debug,
    P::Subpixel: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Atlas")
            .field("textures", &self.textures)
            .field("texcoords", &self.texcoords)
            .finish()
    }
}

/// A texture collection
#[derive(Clone, Default)]
pub struct Textures<P: image::Pixel>(Vec<Texture<P>>);

impl<P: image::Pixel> Textures<P> {
    /// Creates a new texture collection with given parameters.
    #[inline]
    pub fn new_with(page_count: u32, size: u32, mip_level_count: u32) -> Self {
        let textures = (0..page_count)
            .map(|_| Texture::new_with(size, mip_level_count))
            .collect::<Vec<_>>();
        Self(textures)
    }

    /// Extracts an inner value as vec.
    #[inline]
    pub fn into_vec(self) -> Vec<Texture<P>> {
        self.0
    }
}

impl<P> fmt::Debug for Textures<P>
where
    P: image::Pixel + fmt::Debug,
    P::Subpixel: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<P: image::Pixel> ops::Deref for Textures<P> {
    type Target = Vec<Texture<P>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: image::Pixel> ops::DerefMut for Textures<P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: image::Pixel> From<Textures<P>> for Vec<Texture<P>> {
    #[inline]
    fn from(val: Textures<P>) -> Self {
        val.0
    }
}

/// A texture
#[derive(Clone, Default)]
pub struct Texture<P: image::Pixel>(Vec<image::ImageBuffer<P, Vec<P::Subpixel>>>);

impl<P: image::Pixel> Texture<P> {
    /// Creates a new texture with given parameters.
    #[inline]
    pub fn new_with(size: u32, mip_level_count: u32) -> Self {
        let mip_maps = (0..mip_level_count)
            .map(|mip_level| {
                let size = size >> mip_level;
                image::ImageBuffer::new(size, size)
            })
            .collect::<Vec<_>>();
        Self(mip_maps)
    }

    /// Extracts an inner value as vec.
    #[inline]
    pub fn into_vec(self) -> Vec<image::ImageBuffer<P, Vec<P::Subpixel>>> {
        self.0
    }
}

impl<P> fmt::Debug for Texture<P>
where
    P: image::Pixel + fmt::Debug,
    P::Subpixel: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<P: image::Pixel> ops::Deref for Texture<P> {
    type Target = Vec<image::ImageBuffer<P, Vec<P::Subpixel>>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: image::Pixel> ops::DerefMut for Texture<P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: image::Pixel> From<Texture<P>> for Vec<image::ImageBuffer<P, Vec<P::Subpixel>>> {
    fn from(val: Texture<P>) -> Self {
        val.0
    }
}

/// A texture element coordinate representing `u32` position
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Texcoord {
    pub page: u32,
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub size: u32,
}

impl Texcoord {
    /// Returns a normalized texcoord using f32.
    #[inline]
    pub fn to_f32(self) -> Texcoord32 {
        Texcoord32 {
            page: self.page,
            min_x: self.min_x as f32 / self.size as f32,
            min_y: self.min_y as f32 / self.size as f32,
            max_x: self.max_x as f32 / self.size as f32,
            max_y: self.max_y as f32 / self.size as f32,
        }
    }

    /// Returns a normalized texcoord using f64.
    #[inline]
    pub fn to_f64(self) -> Texcoord64 {
        Texcoord64 {
            page: self.page,
            min_x: self.min_x as f64 / self.size as f64,
            min_y: self.min_y as f64 / self.size as f64,
            max_x: self.max_x as f64 / self.size as f64,
            max_y: self.max_y as f64 / self.size as f64,
        }
    }
}

/// A texture element coordinate representing normalized `f32` position
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Texcoord32 {
    pub page: u32,
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl From<Texcoord> for Texcoord32 {
    #[inline]
    fn from(value: Texcoord) -> Self {
        value.to_f32()
    }
}

/// A texture element coordinate representing normalized `f64` position
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Texcoord64 {
    pub page: u32,
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl From<Texcoord> for Texcoord64 {
    #[inline]
    fn from(value: Texcoord) -> Self {
        value.to_f64()
    }
}

/// A texture atlas generation error
#[derive(Debug)]
pub enum AtlasError {
    ZeroMaxPageCount,
    InvalidSize(u32),
    InvalidBlockSize(u32),
    ZeroEntry,
    Packing(rectangle_pack::RectanglePackError),
}

impl fmt::Display for AtlasError {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtlasError::ZeroMaxPageCount => write!(f, "max page count is zero."),
            AtlasError::InvalidSize(size) => write!(f, "size is not power of two: {}.", size),
            AtlasError::InvalidBlockSize(block_size) => write!(f, "block size is not power of two: {}.", block_size),
            AtlasError::ZeroEntry => write!(f, "entry is empty."),
            AtlasError::Packing(err) => err.fmt(f),
        }
    }
}

impl error::Error for AtlasError {}

impl From<rectangle_pack::RectanglePackError> for AtlasError {
    fn from(value: rectangle_pack::RectanglePackError) -> Self {
        AtlasError::Packing(value)
    }
}
