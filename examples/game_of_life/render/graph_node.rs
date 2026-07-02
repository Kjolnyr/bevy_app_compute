use bevy::{
    prelude::*,
    render::{
        render_resource::{CachedPipelineState, Pipeline, PipelineCache, RenderPassDescriptor},
        renderer::{RenderContext, ViewQuery},
        view::ViewTarget,
    },
};

use crate::{bind_groups::ParticleBindGroup, worker::NUMBER_OF_CELLS};

use super::pipeline::DrawParticlePipeline;

pub fn draw_particle_pass(
    _world: &World,
    view: ViewQuery<&ViewTarget>,
    pipeline: Res<DrawParticlePipeline>,
    bindings: Res<ParticleBindGroup>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let target = view.into_inner();

    let color_attachments = [Some(target.get_color_attachment())];

    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: None,
        color_attachments: &color_attachments,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    #[allow(clippy::pattern_type_mismatch)]
    if let CachedPipelineState::Ok(pipeline_cached) =
        pipeline_cache.get_render_pipeline_state(pipeline.pipeline)
    {
        #[allow(clippy::unreachable)]
        let Pipeline::RenderPipeline(pipeline_ready) = pipeline_cached else {
            return;
        };

        pass.set_bind_group(0, &bindings.bind_group, &[]);
        pass.set_render_pipeline(pipeline_ready);
        pass.draw(0..6, 0..NUMBER_OF_CELLS);
    }
}
