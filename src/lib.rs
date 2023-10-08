//! This library provides a texture atlas generation for general purpose.

use std::{
    collections::{BTreeMap, HashMap},
    error, fmt, hash, ops,
};

/// A way of mip generation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtlasMipOption {
    #[default]
    None,
    NoneWithPadding(u32),
    Padding(u32),
    Block(u32),
}

/// A way of texture wrapping.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtlasEntryMipOption {
    #[default]
    Single,
    Repeat,
}

/// A texture atlas entry, which has key and texture.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtlasEntry<K, I: image::GenericImageView> {
    pub key: K,
    pub texture: I,
    pub mip: AtlasEntryMipOption,
}

/// A texture atlas descriptor.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct AtlasDescriptor<'a, K, I: image::GenericImageView> {
    pub max_page_count: u32,
    pub size: u32,
    pub mip: AtlasMipOption,
    pub entries: &'a [AtlasEntry<K, I>],
}

/// Creates a new texture atlas.
pub fn create_atlas<K, I, S>(
    desc: &AtlasDescriptor<'_, K, I>,
) -> Result<Atlas<K, I::Pixel, S>, AtlasError>
where
    K: Clone + Eq + hash::Hash,
    I: image::GenericImage,
    I::Pixel: 'static,
    S: Default + hash::BuildHasher,
{
    match desc.mip {
        AtlasMipOption::None => {
            create_atlas_with_padding(desc.max_page_count, desc.size, false, 0, desc.entries)
        }
        AtlasMipOption::NoneWithPadding(padding) => {
            create_atlas_with_padding(desc.max_page_count, desc.size, false, padding, desc.entries)
        }
        AtlasMipOption::Padding(padding) => {
            create_atlas_with_padding(desc.max_page_count, desc.size, true, padding, desc.entries)
        }
        AtlasMipOption::Block(block_size) => {
            create_atlas_with_block(desc.max_page_count, desc.size, block_size, desc.entries)
        }
    }
}

fn create_atlas_with_padding<K, I, S>(
    max_page_count: u32,
    size: u32,
    mip: bool,
    padding: u32,
    entries: &[AtlasEntry<K, I>],
) -> Result<Atlas<K, I::Pixel, S>, AtlasError>
where
    K: Eq + hash::Hash + Clone,
    I: image::GenericImage,
    I::Pixel: 'static,
    S: hash::BuildHasher + Default,
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

    let page_count = locations
        .packed_locations()
        .iter()
        .map(|(_, (_, location))| location.z())
        .max()
        .unwrap()
        + 1;

    let mip_level_count = if mip { size.ilog2() + 1 } else { 1 };

    let mut textures = Textures::new_with(page_count, size, mip_level_count);
    let mut texcoords = HashMap::default();
    for (&i, (_, location)) in locations.packed_locations() {
        let entry = &entries[i];

        image::imageops::replace(
            &mut textures[location.z() as usize][0],
            &entry_with_padding(&entry.texture, padding, entry.mip),
            location.x() as i64,
            location.y() as i64,
        );

        let texcoord = Texcoord {
            page: location.z(),
            min_x: location.x() + padding,
            min_y: location.y() + padding,
            max_x: location.x() + padding + entry.texture.width(),
            max_y: location.y() + padding + entry.texture.height(),
            size,
        };
        texcoords.insert(entry.key.clone(), texcoord);
    }

    for page in 0..page_count {
        for mip_level in 1..mip_level_count {
            let mip_map = image::imageops::resize(
                &textures[page as usize][0],
                size >> mip_level,
                size >> mip_level,
                image::imageops::FilterType::Triangle,
            );
            image::imageops::replace(
                &mut textures[page as usize][mip_level as usize],
                &mip_map,
                0,
                0,
            );
        }
    }

    Ok(Atlas {
        textures,
        texcoords,
    })
}

fn create_atlas_with_block<K, I, S>(
    max_page_count: u32,
    size: u32,
    block_size: u32,
    entries: &[AtlasEntry<K, I>],
) -> Result<Atlas<K, I::Pixel, S>, AtlasError>
where
    K: Clone + Eq + hash::Hash,
    I: image::GenericImage,
    I::Pixel: 'static,
    S: Default + hash::BuildHasher,
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

    let page_count = locations
        .packed_locations()
        .iter()
        .map(|(_, (_, location))| location.z())
        .max()
        .unwrap()
        + 1;

    let padding = block_size >> 1;
    let mip_level_count = block_size.ilog2() + 1;

    let mut textures = Textures::new_with(page_count, size, mip_level_count);
    let mut texcoords = HashMap::default();
    for (&i, (_, location)) in locations.packed_locations() {
        let entry = &entries[i];

        let texture = entry_with_padding(&entry.texture, padding, entry.mip);

        for mip_level in 0..mip_level_count {
            let mip_map = image::imageops::resize(
                &texture,
                texture.width() >> mip_level,
                texture.height() >> mip_level,
                image::imageops::FilterType::Triangle,
            );

            image::imageops::replace(
                &mut textures[location.z() as usize][mip_level as usize],
                &mip_map,
                (location.x() * block_size >> mip_level) as i64,
                (location.y() * block_size >> mip_level) as i64,
            );
        }

        let texcoord = Texcoord {
            page: location.z(),
            min_x: location.x() * block_size + padding,
            min_y: location.y() * block_size + padding,
            max_x: location.x() * block_size + padding + entry.texture.width(),
            max_y: location.y() * block_size + padding + entry.texture.height(),
            size,
        };
        texcoords.insert(entry.key.clone(), texcoord);
    }

    Ok(Atlas {
        textures,
        texcoords,
    })
}

fn entry_with_padding<I>(
    src: &I,
    padding: u32,
    leak: AtlasEntryMipOption,
) -> image::ImageBuffer<I::Pixel, Vec<<I::Pixel as image::Pixel>::Subpixel>>
where
    I: image::GenericImage,
{
    match leak {
        AtlasEntryMipOption::Single => {
            let mut target =
                image::ImageBuffer::new(src.width() + padding * 2, src.height() + padding * 2);
            image::imageops::replace(&mut target, src, padding as i64, padding as i64);
            target
        }
        AtlasEntryMipOption::Repeat => {
            let mut target =
                image::ImageBuffer::new(src.width() + padding * 2, src.height() + padding * 2);
            for x in -1..=1 {
                for y in -1..=1 {
                    let x = padding as i32 + src.width() as i32 * x;
                    let y = padding as i32 + src.height() as i32 * y;
                    image::imageops::replace(&mut target, src, x as i64, y as i64);
                }
            }
            target
        }
    }
}

/// A texture atlas, which has textures and coordinates.
#[derive(Clone, Default)]
pub struct Atlas<K, P: image::Pixel, S> {
    pub textures: Textures<P>,
    pub texcoords: HashMap<K, Texcoord, S>,
}

impl<K, P, S> fmt::Debug for Atlas<K, P, S>
where
    K: fmt::Debug,
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

/// A texture collection, which has some texture.
#[derive(Clone, Default)]
pub struct Textures<P: image::Pixel>(Vec<Texture<P>>);

impl<P: image::Pixel> Textures<P> {
    /// Creates a new textures with parameters.
    #[inline]
    pub fn new_with(page_count: u32, size: u32, mip_level_count: u32) -> Self {
        let textures = (0..page_count)
            .map(|_| Texture::new_with(size, mip_level_count))
            .collect::<Vec<_>>();
        Self(textures)
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

/// A texture, which has some mip maps.
#[derive(Clone, Default)]
pub struct Texture<P: image::Pixel>(Vec<image::ImageBuffer<P, Vec<P::Subpixel>>>);

impl<P: image::Pixel> Texture<P> {
    /// Creates a new texture with parameters.
    #[inline]
    pub fn new_with(size: u32, mip_level_count: u32) -> Self {
        let mip_maps = (0..mip_level_count)
            .map(|mip_level| image::ImageBuffer::new(size >> mip_level, size >> mip_level))
            .collect::<Vec<_>>();
        Self(mip_maps)
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

/// A texture coordinate.
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

/// A texture coordinate based on f32.
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

/// A texture coordinate based on f64.
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

/// An error for texture atlas generation.
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
