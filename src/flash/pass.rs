use std::ops::Range;

use amethyst::{
    assets::{
        Loader,
        AssetStorage,
        Handle,
    },
    core::ecs::{
        DispatcherBuilder, World,
    },
    error::Error,
    renderer::{
        formats::texture::ImageFormat,
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::{Factory, ImageState},
            graph::{
                GraphContext,
                NodeBuffer, NodeImage, render::{PrepareResult, RenderGroup, RenderGroupDesc},
            },
            hal::{self, device::Device,  pso, pso::ShaderStageFlags},
            mesh::{AsVertex, Position, TexCoord, PosTex},
            shader::{Shader, SpirvShader},
            texture::{TextureBuilder, pixel::Rgba8Srgb},
        },
        submodules::{
            FlatEnvironmentSub,
            TextureSub,
            TextureId,
        },
        Texture,
        types::Backend, util,
    },
};

use std::io::Cursor;
use super::*;
use crate::{
    flash::sub::*,
};

use crate::renderutils::*;

use amethyst::prelude::WorldExt;
use crate::flash::sub::FlashSub;
use image::load;

const STATIC_DEPTH: f32 = 0.0;
const STATIC_CROP: f32 = 0.0;

const STATIC_VERTEX_DATA: [PosTex; 4] = [
    PosTex { position: Position([-1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 0.0 + STATIC_CROP]) },
    PosTex { position: Position([-1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 0.0 + STATIC_CROP]) },
];

const STATIC_INSTANCE_DATA: [u32; 6] = [0, 1, 2, 0, 3, 2];


lazy_static::lazy_static! {
    // These uses the precompiled shaders.
    // These can be obtained using glslc.exe in the vulkan sdk.
    static ref VERTEX: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/flash.vert.spv"),
        ShaderStageFlags::VERTEX,
        "main",
    ).unwrap();

    static ref FRAGMENT: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/flash.frag.spv"),
        ShaderStageFlags::FRAGMENT,
        "main",
    ).unwrap();
}

/// Draw triangles.
#[derive(Debug, Copy, Clone, Default)]
pub struct DrawFlashDesc;

impl<B: Backend> RenderGroupDesc<B, World> for DrawFlashDesc {
    fn build(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        world: &World,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
        let env = FlatEnvironmentSub::new(factory)?;
        let flashes = FlashSub::new(
            factory,
            hal::pso::ShaderStageFlags::VERTEX | hal::pso::ShaderStageFlags::FRAGMENT
        )?;
        let mut tex = TextureSub::new(factory)?;
        
        // Load billboard mesh.
        let vertex = StaticVertexBuffer::new();
        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), flashes.raw_layout(), tex.raw_layout()],
            None,
        )?;


        Ok(Box::new(DrawFlash::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            flashes,
            tex,
        }))
    }
}

/// Draws triangles to the screen.
#[derive(Debug)]
pub struct DrawFlash<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: FlatEnvironmentSub<B>,
    vertex: StaticVertexBuffer<B, PosTex>,
    flashes: FlashSub<B>,
    tex: TextureSub<B>,
    // query_pool: B::QueryPool,
}

impl<B: Backend> RenderGroup<B, World> for DrawFlash<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {

        self.env.process(factory, index, world);
        self.flashes.process(factory, index, world);

        // Load any unloaded textures.
        if let Some(mut flash_texture) = world.try_fetch_mut::<FlashTexture>() {
            if let Some((texture, b)) = self.tex.insert(factory, world, &flash_texture.texture, hal::image::Layout::ShaderReadOnlyOptimal) {
                flash_texture.tex_id = Some(texture);
            }
        }
        self.tex.maintain(factory, world);

        self.vertex.prepare(factory, queue, &STATIC_VERTEX_DATA, Some(&STATIC_INSTANCE_DATA), index).expect("Failed to allocate static vertex buffer!");

        PrepareResult::DrawRecord
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) {

        if !self.flashes.is_empty() {

            encoder.bind_graphics_pipeline(&self.pipeline);
            self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);
            self.flashes.bind(index, &self.pipeline_layout, 1, &mut encoder);

            if let Some(mut flash_texture) = world.try_fetch::<FlashTexture>() {
                if let Some(texture_id) = flash_texture.tex_id {
                    if self.tex.loaded(texture_id) {
                        self.tex.bind(&self.pipeline_layout, 2, texture_id, &mut encoder);
                        unsafe {
                            self.vertex.draw(&mut encoder, 0..self.flashes.count() as u32, index);
                        }
                    }

                }
            }

        }

    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _world: &World) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_custom_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
    push_constant: Option<(hal::pso::ShaderStageFlags, Range<u32>)>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, push_constant)
    }?;
    // Load the shaders
    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

    // Build the pipeline
    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                // This Pipeline uses our custom vertex description and uses instancing.
                .with_vertex_desc(&[(PosTex::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
                // Add the shaders
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_depth_test(pso::DepthTest {
                    fun: pso::Comparison::Less,
                    write: true,
                })
                .with_blend_targets(vec![pso::ColorBlendDesc { blend: Some(pso::BlendState::ALPHA), mask: pso::ColorMask::ALL}]),
        )
        .build(factory, None);

    // Destoy the shaders once loaded
    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }

    // Handle the Errors
    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}

/// A [RenderPlugin] for our custom plugin
#[derive(Debug)]
pub struct FlashRender {
    flash_path: String,
}

impl FlashRender {
    pub fn new(flash_path: impl Into<String>) -> Self {
        Self {
            flash_path: flash_path.into(),
        }
    }
}

impl<B: Backend> RenderPlugin<B> for FlashRender {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        let tex = {
            if !world.has_value::<AssetStorage::<Texture>>() {
                world.insert(AssetStorage::<Texture>::new());
            }
            let loader = world.read_resource::<Loader>();
            loader.load(
                &self.flash_path,
                amethyst::renderer::formats::texture::ImageFormat::default(),
                (),
                &world.read_resource::<AssetStorage<Texture>>(),
            )
        };

        world.insert(FlashTexture::new(tex));

        // Add the required components to the world ECS
        // We need to move the object out of the option to obtain it validly.
        world.register::<crate::flash::Flash>();
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            // Add our Description
            ctx.add(RenderOrder::LinearPostEffects, DrawFlashDesc::default().builder())?;
            Ok(())
        });
        Ok(())
    }
}