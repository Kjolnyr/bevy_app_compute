//! A custom render node for drawing particles as simple pixels

use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext, RenderLabel},
        render_resource::{CachedPipelineState, Pipeline, PipelineCache, RenderPassDescriptor},
        renderer::RenderContext,
        view::ViewTarget,
    },
};

use crate::{bind_groups::ParticleBindGroup, worker::NUMBER_OF_CELLS};

use super::pipeline::DrawParticlePipeline;

/// The label for our custom node in the render graph
#[derive(RenderLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawParticleLabel;

/// Our custom render node in the render graph
#[derive(Default)]
pub struct DrawParticleNode;

#[allow(clippy::missing_trait_methods)]
impl render_graph::ViewNode for DrawParticleNode {
    type ViewQuery = &'static ViewTarget;

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<DrawParticlePipeline>();
        let bindings = world.resource::<ParticleBindGroup>();

        let color_attachment = view_target.get_color_attachment();

        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        // TODO: I couldn't figure out how to match on the reference.
        #[allow(clippy::pattern_type_mismatch)]
        if let CachedPipelineState::Ok(pipeline_cached) =
            pipeline_cache.get_render_pipeline_state(pipeline.pipeline)
        {
            #[allow(clippy::unreachable)]
            let Pipeline::RenderPipeline(pipeline_ready) = pipeline_cached
            else {
                unreachable!("Cached pipeline isn't ready");
            };

            pass.set_bind_group(0, &bindings.bind_group, &[]);
            pass.set_pipeline(pipeline_ready);
            pass.draw(0..6, 0..NUMBER_OF_CELLS);
        }

        Ok(())
    }
}
