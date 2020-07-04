pub mod sub;
pub mod pass;
use amethyst::{
    assets::{
        PrefabData,
        Handle,
    },
    derive::PrefabData,
    core::math::{
        Vector3,
    },
    renderer::{
        Texture,
        submodules::TextureId,
    },
    error::Error,
    ecs::prelude::*,
};

use serde::{Serialize, Deserialize};

use glsl_layout::*;

#[derive(Debug, Clone, PartialEq)]
pub struct FlashTexture {
    texture: Handle<Texture>,
    tex_id: Option<TextureId>,
}

impl FlashTexture {
    pub fn new(texture: Handle<Texture>) -> Self {
        Self {
            texture,
            tex_id: None,
        }
    }
}

#[derive(Debug, Clone, Default, Component)]
pub struct Flash;

pub const MAX_FLASHES: usize = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct FlashData {
    pub center: vec3,
    pub scale: float,
}

impl FlashData {
    pub(crate) fn new(center: Vector3<f32>, scale: f32) -> Self {
        Self {
            center: Into::<[f32; 3]>::into(center).into(),
            scale,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct FlashList {
    count: uint,
    stars: [FlashData; MAX_FLASHES],
}

impl FlashList {
    pub(crate) fn new(star_data: &[FlashData]) -> Self {
        assert!(star_data.len() <= MAX_FLASHES);
        let mut stars: [FlashData; MAX_FLASHES] = Default::default();
        for (i, data) in star_data.iter().enumerate() {
            stars[i] = *data;
        }
        Self { stars, count: star_data.len() as u32 }
    }
}